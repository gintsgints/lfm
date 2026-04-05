use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub fn zip_paths(sources: &[PathBuf], dest: &Path) -> io::Result<()> {
    let file = File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for src in sources {
        let base = src.parent().unwrap_or(Path::new("."));
        add_to_zip(&mut zip, src, base, options)?;
    }
    zip.finish().map_err(io::Error::other)?;
    Ok(())
}

fn add_to_zip(
    zip: &mut zip::ZipWriter<File>,
    path: &Path,
    base: &Path,
    options: zip::write::SimpleFileOptions,
) -> io::Result<()> {
    let rel = path.strip_prefix(base).unwrap_or(path);
    let name = rel.to_string_lossy();

    if path.is_dir() {
        zip.add_directory(format!("{name}/"), options)
            .map_err(io::Error::other)?;
        for entry in std::fs::read_dir(path)?.filter_map(std::result::Result::ok) {
            add_to_zip(zip, &entry.path(), base, options)?;
        }
    } else {
        zip.start_file(name.as_ref(), options)
            .map_err(io::Error::other)?;
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }
    Ok(())
}

pub fn unzip(src: &Path, dest_dir: &Path) -> io::Result<()> {
    let file = File::open(src)?;
    let mut archive = zip::ZipArchive::new(file).map_err(io::Error::other)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(io::Error::other)?;
        let out_path = dest_dir.join(entry.mangled_name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = File::create(&out_path)?;
            io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

pub fn untar_gz(src: &Path, dest_dir: &Path) -> io::Result<()> {
    let file = File::open(src)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(dest_dir)?;
    Ok(())
}
