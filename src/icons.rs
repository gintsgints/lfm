//! Maps file names to Nerd Font glyphs based on their extension.
//!
//! Icons require a patched "Nerd Font" in the terminal; with a plain font the
//! glyphs render as a fallback box, but the file name beside them stays legible.

use std::path::Path;

/// Glyph shown for directories.
pub const DIR: &str = "\u{f024b}"; // 󰉋

/// Fallback glyph for files with no known extension.
pub const FILE: &str = "\u{f0219}"; // 󰈙

/// Returns the icon glyph for a file name, chosen by its (lowercased)
/// extension. Falls back to [`FILE`] for unknown or extensionless names.
#[must_use]
pub fn for_name(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        // Languages
        "rs" => "\u{e7a8}",                                //
        "py" | "pyc" | "pyo" => "\u{e606}",                //
        "js" | "mjs" | "cjs" => "\u{e60c}",                //
        "ts" => "\u{e628}",                                //
        "jsx" | "tsx" => "\u{e7ba}",                       //
        "go" => "\u{e627}",                                //
        "c" | "h" => "\u{e61e}",                           //
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "\u{e61d}", //
        "java" | "class" | "jar" => "\u{e738}",            //
        "rb" => "\u{e739}",                                //
        "php" => "\u{e73d}",                               //
        "swift" => "\u{e755}",                             //
        "kt" | "kts" => "\u{e634}",                        //
        "lua" => "\u{e620}",                               //
        "sh" | "bash" | "zsh" | "fish" => "\u{ebca}",      //
        "vim" => "\u{e62b}",                               //

        // Web / markup
        "html" | "htm" => "\u{e736}",    //
        "css" => "\u{e749}",             //
        "scss" | "sass" => "\u{e603}",   //
        "md" | "markdown" => "\u{f48a}", //
        "xml" => "\u{f05c0}",            // 󰗀

        // Data / config
        "json" => "\u{e60b}",                 //
        "toml" => "\u{e6b2}",                 //
        "yaml" | "yml" => "\u{e6a8}",         //
        "ini" | "cfg" | "conf" => "\u{e615}", //
        "lock" => "\u{f023}",                 //
        "csv" | "tsv" => "\u{f021b}",         // 󰈛

        // Documents
        "txt" | "text" => "\u{f0219}", // 󰈙
        "pdf" => "\u{f1c1}",           //
        "doc" | "docx" => "\u{f1c2}",  //
        "xls" | "xlsx" => "\u{f1c3}",  //
        "ppt" | "pptx" => "\u{f1c4}",  //

        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tiff" => "\u{f1c5}", //
        "svg" => "\u{f0721}",                                                           // 󰜡

        // Audio / video
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "\u{f001}", //
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv" => "\u{f03d}", //

        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" => "\u{f1c6}", //

        // VCS
        "gitignore" | "gitattributes" | "gitmodules" => "\u{e702}", //

        _ => FILE,
    }
}
