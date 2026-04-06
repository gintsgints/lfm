use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

pub enum ProgressMsg {
    Tick { current: u64, total: u64 },
    Done,
}

pub fn run_copy(sources: &[PathBuf], dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    // Announce total before the first file is copied so the bar shows "0 / N".
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    for src in sources {
        copy_entry(src, dst, &mut current, total, tx);
    }
    let _ = tx.send(ProgressMsg::Done);
}

pub fn run_move(sources: &[PathBuf], dst: &Path, tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    for src in sources {
        move_entry(src, dst, &mut current, total, tx);
    }
    let _ = tx.send(ProgressMsg::Done);
}

pub fn run_delete(sources: &[PathBuf], tx: &mpsc::Sender<ProgressMsg>) {
    let total = count_files(sources);
    let _ = tx.send(ProgressMsg::Tick { current: 0, total });
    let mut current = 0u64;
    for src in sources {
        // Count before deleting — can't walk the tree after it's gone.
        let file_count = count_path(src);
        if src.is_dir() {
            std::fs::remove_dir_all(src).ok();
        } else {
            std::fs::remove_file(src).ok();
        }
        advance(&mut current, file_count, total, tx);
    }
    let _ = tx.send(ProgressMsg::Done);
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
) {
    let Some(name) = src.file_name() else { return };
    let dst = dst_dir.join(name);
    if src.is_dir() {
        if std::fs::create_dir_all(&dst).is_ok() {
            copy_dir(src, &dst, current, total, tx);
        } else {
            // Failed to create destination dir; skip but still advance counter.
            advance(current, count_path(src), total, tx);
        }
    } else {
        std::fs::copy(src, &dst).ok();
        tick(current, total, tx);
    }
}

fn copy_dir(src: &Path, dst: &Path, current: &mut u64, total: u64, tx: &mpsc::Sender<ProgressMsg>) {
    let Ok(rd) = std::fs::read_dir(src) else {
        return;
    };
    for entry in rd.filter_map(std::result::Result::ok) {
        let dst_path = dst.join(entry.file_name());
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if std::fs::create_dir_all(&dst_path).is_ok() {
                copy_dir(&entry.path(), &dst_path, current, total, tx);
            } else {
                advance(current, count_path(&entry.path()), total, tx);
            }
        } else {
            std::fs::copy(entry.path(), &dst_path).ok();
            tick(current, total, tx);
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
) {
    let Some(name) = src.file_name() else { return };
    let dst = dst_dir.join(name);

    // Count files in this source item before attempting rename so we can
    // advance the counter accurately even when rename is instant.
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
        if std::fs::create_dir_all(&dst).is_ok() {
            copy_dir(src, &dst, current, total, tx);
        } else {
            advance(current, file_count, total, tx);
        }
        std::fs::remove_dir_all(src).ok();
    } else {
        std::fs::copy(src, &dst).ok();
        tick(current, total, tx);
        std::fs::remove_file(src).ok();
    }
}

// ---

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
