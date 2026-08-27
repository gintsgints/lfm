use std::path::Path;

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

/// A comma-separated list of glob patterns restricting which files a search
/// looks at.
///
/// A pattern containing `/` is matched against the path relative to the search
/// root (`src/**/*.rs`); one without is matched against the file name alone, so
/// `*.rs` hits every Rust file at any depth. An entry passes when any pattern
/// matches.
///
/// Patterns that fail to compile are skipped rather than rejected: the mask is
/// re-parsed on every keystroke, and a half-typed pattern must not blank the
/// panel.
pub struct FileMask {
    /// Patterns matched against the file name.
    name: Option<GlobSet>,
    /// Patterns matched against the path relative to the search root.
    path: Option<GlobSet>,
}

impl FileMask {
    pub fn parse(text: &str) -> Self {
        let mut names = GlobSetBuilder::new();
        let mut paths = GlobSetBuilder::new();
        let (mut has_name, mut has_path) = (false, false);

        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if part.contains('/') {
                // `literal_separator` keeps `*` inside one path component, so
                // `src/*.rs` does not reach into subdirectories while `**` does.
                if let Ok(glob) = GlobBuilder::new(part).literal_separator(true).build() {
                    paths.add(glob);
                    has_path = true;
                }
            } else if let Ok(glob) = Glob::new(part) {
                names.add(glob);
                has_name = true;
            }
        }

        Self {
            name: has_name.then(|| names.build().ok()).flatten(),
            path: has_path.then(|| paths.build().ok()).flatten(),
        }
    }

    /// Whether the mask carries no usable pattern, in which case it matches
    /// everything and the search can skip filtering entirely.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.path.is_none()
    }

    pub fn matches(&self, rel_path: &Path) -> bool {
        if self.is_empty() {
            return true;
        }
        let name_hit = self.name.as_ref().is_some_and(|set| {
            rel_path
                .file_name()
                .is_some_and(|name| set.is_match(Path::new(name)))
        });
        name_hit || self.path.as_ref().is_some_and(|set| set.is_match(rel_path))
    }
}
