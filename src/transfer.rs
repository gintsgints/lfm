use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

pub enum ProgressMsg {
    Tick { current: u64, total: u64 },
    Done { error: Option<String> },
}

pub fn run_copy(sources: &[PathBuf], dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    let mut first_error: Option<String> = None;
    for src in sources {
        copy_entry(src, dst, &mut current, total, tx, &mut first_error);
    }
    let _ = tx.send(ProgressMsg::Done { error: first_error });
}

pub fn run_move(sources: &[PathBuf], dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    let mut first_error: Option<String> = None;
    for src in sources {
        move_entry(src, dst, &mut current, total, tx, &mut first_error);
    }
    let _ = tx.send(ProgressMsg::Done { error: first_error });
}

pub fn run_copy_rename(src: &Path, dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_path(src);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    let mut first_error: Option<String> = None;
    copy_to(src, dst, &mut current, total, tx, &mut first_error);
    let _ = tx.send(ProgressMsg::Done { error: first_error });
}

pub fn run_move_rename(src: &Path, dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_path(src);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    let mut first_error: Option<String> = None;
    move_to(src, dst, &mut current, total, tx, &mut first_error);
    let _ = tx.send(ProgressMsg::Done { error: first_error });
}

pub fn run_delete(sources: &[PathBuf], tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    let mut first_error: Option<String> = None;
    for src in sources {
        let file_count = count_path(src);
        let result = if src.is_dir() {
            std::fs::remove_dir_all(src)
        } else {
            std::fs::remove_file(src)
        };
        if let Err(e) = result {
            record_error(&mut first_error, format!("{}: {e}", src.display()));
        }
        advance(&mut current, file_count, total, tx);
    }
    let _ = tx.send(ProgressMsg::Done { error: first_error });
}

// --- file counting ---

fn count_files(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|p| count_path(p)).sum()
}

fn count_path(path: &Path) -> u64 {
    if !path.is_dir() {
        return 1;
    }
    let Ok(rd) = std::fs::read_dir(path) else {
        return 0;
    };
    rd.filter_map(std::result::Result::ok)
        .map(|e| count_path(&e.path()))
        .sum()
}

// --- copy ---

fn copy_entry(
    src: &Path,
    dst_dir: &Path,
    current: &mut u64,
    total: u64,
    tx: &mpsc::Sender<ProgressMsg>,
    err: &mut Option<String>,
) {
    let Some(name) = src.file_name() else { return };
    let dst = dst_dir.join(name);
    if src.is_dir() {
        match std::fs::create_dir_all(&dst) {
            Ok(()) => copy_dir(src, &dst, current, total, tx, err),
            Err(e) => {
                record_error(err, format!("{}: {e}", dst.display()));
                advance(current, count_path(src), total, tx);
            }
        }
    } else {
        match std::fs::copy(src, &dst) {
            Ok(_) => tick(current, total, tx),
            Err(e) => {
                record_error(err, format!("{}: {e}", src.display()));
                tick(current, total, tx);
            }
        }
    }
}

fn copy_dir(
    src: &Path,
    dst: &Path,
    current: &mut u64,
    total: u64,
    tx: &mpsc::Sender<ProgressMsg>,
    err: &mut Option<String>,
) {
    let Ok(rd) = std::fs::read_dir(src) else {
        return;
    };
    for entry in rd.filter_map(std::result::Result::ok) {
        let dst_path = dst.join(entry.file_name());
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            match std::fs::create_dir_all(&dst_path) {
                Ok(()) => copy_dir(&entry.path(), &dst_path, current, total, tx, err),
                Err(e) => {
                    record_error(err, format!("{}: {e}", dst_path.display()));
                    advance(current, count_path(&entry.path()), total, tx);
                }
            }
        } else {
            match std::fs::copy(entry.path(), &dst_path) {
                Ok(_) => tick(current, total, tx),
                Err(e) => {
                    record_error(err, format!("{}: {e}", entry.path().display()));
                    tick(current, total, tx);
                }
            }
        }
    }
}

// --- move ---

fn move_entry(
    src: &Path,
    dst_dir: &Path,
    current: &mut u64,
    total: u64,
    tx: &mpsc::Sender<ProgressMsg>,
    err: &mut Option<String>,
) {
    let Some(name) = src.file_name() else { return };
    let dst = dst_dir.join(name);
    let file_count = count_path(src);

    if std::fs::rename(src, &dst).is_ok() {
        *current += file_count;
        let _ = tx.send(ProgressMsg::Tick {
            current: *current,
            total,
        });
        return;
    }

    // Cross-device fallback: copy then delete.
    if src.is_dir() {
        match std::fs::create_dir_all(&dst) {
            Ok(()) => copy_dir(src, &dst, current, total, tx, err),
            Err(e) => {
                record_error(err, format!("{}: {e}", dst.display()));
                advance(current, file_count, total, tx);
            }
        }
        std::fs::remove_dir_all(src).ok();
    } else {
        match std::fs::copy(src, &dst) {
            Ok(_) => tick(current, total, tx),
            Err(e) => {
                record_error(err, format!("{}: {e}", src.display()));
                tick(current, total, tx);
            }
        }
        std::fs::remove_file(src).ok();
    }
}

// --- copy/move to an exact destination path (used for rename operations) ---

fn copy_to(
    src: &Path,
    dst: &Path,
    current: &mut u64,
    total: u64,
    tx: &mpsc::Sender<ProgressMsg>,
    err: &mut Option<String>,
) {
    if src.is_dir() {
        match std::fs::create_dir_all(dst) {
            Ok(()) => copy_dir(src, dst, current, total, tx, err),
            Err(e) => {
                record_error(err, format!("{}: {e}", dst.display()));
                advance(current, count_path(src), total, tx);
            }
        }
    } else {
        match std::fs::copy(src, dst) {
            Ok(_) => tick(current, total, tx),
            Err(e) => {
                record_error(err, format!("{}: {e}", src.display()));
                tick(current, total, tx);
            }
        }
    }
}

fn move_to(
    src: &Path,
    dst: &Path,
    current: &mut u64,
    total: u64,
    tx: &mpsc::Sender<ProgressMsg>,
    err: &mut Option<String>,
) {
    let file_count = count_path(src);
    if std::fs::rename(src, dst).is_ok() {
        *current += file_count;
        let _ = tx.send(ProgressMsg::Tick {
            current: *current,
            total,
        });
        return;
    }
    // Cross-device fallback: copy then delete.
    if src.is_dir() {
        match std::fs::create_dir_all(dst) {
            Ok(()) => copy_dir(src, dst, current, total, tx, err),
            Err(e) => {
                record_error(err, format!("{}: {e}", dst.display()));
                advance(current, file_count, total, tx);
            }
        }
        std::fs::remove_dir_all(src).ok();
    } else {
        match std::fs::copy(src, dst) {
            Ok(_) => tick(current, total, tx),
            Err(e) => {
                record_error(err, format!("{}: {e}", src.display()));
                tick(current, total, tx);
            }
        }
        std::fs::remove_file(src).ok();
    }
}

// ---

fn record_error(first_error: &mut Option<String>, msg: String) {
    if first_error.is_none() {
        *first_error = Some(msg);
    }
}

fn tick(current: &mut u64, total: u64, tx: &mpsc::Sender<ProgressMsg>) {
    advance(current, 1, total, tx);
}

fn advance(current: &mut u64, by: u64, total: u64, tx: &mpsc::Sender<ProgressMsg>) {
    *current += by;
    let _ = tx.send(ProgressMsg::Tick {
        current: *current,
        total,
    });
}
