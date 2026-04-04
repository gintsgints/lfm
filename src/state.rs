use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub left_dir: Option<PathBuf>,
    pub pins: Vec<PathBuf>,
}

pub fn load() -> PersistedState {
    state_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(state: &PersistedState) -> io::Result<()> {
    let path = state_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not determine state path"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state).map_err(io::Error::other)?;
    fs::write(path, json)
}

fn state_path() -> Option<PathBuf> {
    dirs_base().map(|d| d.join("lfm").join("state.json"))
}

fn dirs_base() -> Option<PathBuf> {
    // $XDG_CONFIG_HOME, then ~/.config
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
