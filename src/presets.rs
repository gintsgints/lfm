//! User-defined preset commands loaded from `~/.config/lfm/commands.json`.
//!
//! A preset's `command` is either an array (argv mode — passed directly to the
//! OS) or a string (shell mode — passed to `sh -c`). Placeholders `{files}`,
//! `{paths}`, and `{input}` are substituted at run time from the current
//! selection and an optional prompt. See [`Preset::expand`] for the rules.

use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Suspend the TUI, run the command attached to the terminal, then wait
    /// for a keypress before restoring.
    Block,
    /// Run the command with stdout/stderr piped, then show the merged output
    /// in a scrollable popup.
    Capture,
    /// Spawn the command and return immediately. Output is discarded.
    #[default]
    Background,
}

/// Either an argv array (preferred) or a shell command string passed to
/// `sh -c`. Argv mode does not invoke a shell — no quoting/escaping pitfalls.
#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CommandTemplate {
    Argv(Vec<String>),
    Shell(String),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Preset {
    pub label: String,
    pub command: CommandTemplate,
    #[serde(default)]
    pub output: OutputMode,
}

#[derive(Default, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    presets: Vec<Preset>,
}

/// What the runtime hands to `std::process::Command` after placeholder
/// substitution.
pub enum ExecSpec {
    /// Direct argv — no shell.
    Argv(Vec<OsString>),
    /// Single string for `sh -c <string>`.
    Shell(String),
}

impl Preset {
    /// Whether the template references `{input}` anywhere. Used to decide if
    /// the picker should prompt for a value before running.
    pub fn needs_input(&self) -> bool {
        self.template_contains("{input}")
    }

    /// Whether the template references `{files}` or `{paths}`. Used to decide
    /// whether an empty selection is an error.
    pub fn references_files(&self) -> bool {
        self.template_contains("{files}") || self.template_contains("{paths}")
    }

    fn template_contains(&self, needle: &str) -> bool {
        match &self.command {
            CommandTemplate::Argv(parts) => parts.iter().any(|p| p.contains(needle)),
            CommandTemplate::Shell(s) => s.contains(needle),
        }
    }

    /// Substitute the selection and optional `input` into the template.
    ///
    /// `names` are entry names (relative to the panel cwd, as the user sees
    /// them on screen); `paths` are absolute paths in the same order.
    ///
    /// Argv mode: when a placeholder *is* a whole arg, it expands to N argv
    /// entries (one per file). When embedded inside a larger arg, the
    /// selection must be exactly one file.
    ///
    /// Shell mode: each path is single-quoted and joined with spaces; `{input}`
    /// is single-quoted as one value.
    pub fn expand(
        &self,
        names: &[String],
        paths: &[PathBuf],
        cwd: &Path,
        input: &str,
    ) -> Result<ExecSpec, String> {
        match &self.command {
            CommandTemplate::Argv(parts) => {
                expand_argv(parts, names, paths, cwd, input).map(ExecSpec::Argv)
            }
            CommandTemplate::Shell(s) => {
                Ok(ExecSpec::Shell(expand_shell(s, names, paths, cwd, input)))
            }
        }
    }

    /// Single-line preview used by the picker — joins argv with spaces, or
    /// shows the shell string as-is.
    pub fn command_preview(&self) -> String {
        match &self.command {
            CommandTemplate::Argv(parts) => parts.join(" "),
            CommandTemplate::Shell(s) => s.clone(),
        }
    }
}

fn expand_argv(
    parts: &[String],
    names: &[String],
    paths: &[PathBuf],
    cwd: &Path,
    input: &str,
) -> Result<Vec<OsString>, String> {
    let cwd_s = cwd.display().to_string();
    let mut out: Vec<OsString> = Vec::new();
    for part in parts {
        if part == "{files}" {
            for n in names {
                out.push(OsString::from(n));
            }
        } else if part == "{paths}" {
            for p in paths {
                out.push(p.clone().into_os_string());
            }
        } else if part.contains("{files}") || part.contains("{paths}") {
            // Embedded multi-value placeholder — only valid for a single file.
            if names.len() != 1 {
                return Err(format!(
                    "placeholder embedded in '{part}' requires exactly one selected file (got {})",
                    names.len()
                ));
            }
            let s = substitute_single(part, Some(&names[0]), Some(&paths[0]), &cwd_s, input);
            out.push(OsString::from(s));
        } else {
            // {cwd} and {input} are always single values, so they're safe to
            // substitute on any arg regardless of selection size.
            out.push(OsString::from(substitute_single(
                part, None, None, &cwd_s, input,
            )));
        }
    }
    Ok(out)
}

/// Replace single-value placeholders in `s`. `name` and `path` are `Some` only
/// when we've verified there's exactly one selected file.
fn substitute_single(
    s: &str,
    name: Option<&String>,
    path: Option<&PathBuf>,
    cwd: &str,
    input: &str,
) -> String {
    let mut out = s.replace("{cwd}", cwd).replace("{input}", input);
    if let (Some(name), Some(path)) = (name, path) {
        out = out
            .replace("{files}", name)
            .replace("{paths}", &path.display().to_string());
    }
    out
}

fn expand_shell(
    template: &str,
    names: &[String],
    paths: &[PathBuf],
    cwd: &Path,
    input: &str,
) -> String {
    let files = names
        .iter()
        .map(|n| shell_quote(n))
        .collect::<Vec<_>>()
        .join(" ");
    let pathstr = paths
        .iter()
        .map(|p| shell_quote(&p.display().to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    let cwd_q = shell_quote(&cwd.display().to_string());
    let input_q = shell_quote(input);
    template
        .replace("{files}", &files)
        .replace("{paths}", &pathstr)
        .replace("{cwd}", &cwd_q)
        .replace("{input}", &input_q)
}

/// POSIX single-quote wrapping: encloses `s` in `'…'` and escapes any embedded
/// single quotes as `'\''`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Resolve `~/.config/lfm/commands.json` (respecting `XDG_CONFIG_HOME`).
pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("lfm").join("commands.json"))
}

/// Load presets from disk. If the file is missing, write a starter file and
/// load that. Returns the parsed preset list or a human-readable error.
pub fn load_or_create() -> Result<Vec<Preset>, String> {
    let path = config_path().ok_or_else(|| "could not determine config path".to_string())?;
    if !path.exists() {
        write_starter(&path)
            .map_err(|e| format!("write starter config to {}: {e}", path.display()))?;
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let cfg: Config =
        serde_json::from_str(&content).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(cfg.presets)
}

fn write_starter(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, STARTER_JSON)
}

const STARTER_JSON: &str = r#"{
  "presets": [
    {
      "label": "file (show type)",
      "command": ["file", "{paths}"],
      "output": "capture"
    },
    {
      "label": "wc (count lines)",
      "command": ["wc", "-l", "{paths}"],
      "output": "capture"
    },
    {
      "label": "grep (search inside selection for {input})",
      "command": "grep -rn {input} {files}",
      "output": "capture"
    }
  ]
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    fn paths(v: &[&str]) -> Vec<PathBuf> {
        v.iter().map(PathBuf::from).collect()
    }

    fn cwd() -> &'static Path {
        Path::new("/cwd")
    }

    fn argv_preset(parts: &[&str]) -> Preset {
        Preset {
            label: "t".into(),
            command: CommandTemplate::Argv(names(parts)),
            output: OutputMode::Block,
        }
    }

    fn shell_preset(s: &str) -> Preset {
        Preset {
            label: "t".into(),
            command: CommandTemplate::Shell(s.into()),
            output: OutputMode::Block,
        }
    }

    fn expect_argv(spec: ExecSpec) -> Vec<String> {
        match spec {
            ExecSpec::Argv(v) => v
                .into_iter()
                .map(|o| o.to_string_lossy().into_owned())
                .collect(),
            ExecSpec::Shell(_) => panic!("expected argv"),
        }
    }

    fn expect_shell(spec: ExecSpec) -> String {
        match spec {
            ExecSpec::Shell(s) => s,
            ExecSpec::Argv(_) => panic!("expected shell"),
        }
    }

    #[test]
    fn argv_files_expands_to_multiple_args() {
        let p = argv_preset(&["ls", "{files}"]);
        let got = expect_argv(
            p.expand(&names(&["a", "b"]), &paths(&["/x/a", "/x/b"]), cwd(), "")
                .unwrap(),
        );
        assert_eq!(got, vec!["ls", "a", "b"]);
    }

    #[test]
    fn argv_paths_expands_to_absolute() {
        let p = argv_preset(&["cat", "{paths}"]);
        let got = expect_argv(
            p.expand(&names(&["a"]), &paths(&["/x/a"]), cwd(), "")
                .unwrap(),
        );
        assert_eq!(got, vec!["cat", "/x/a"]);
    }

    #[test]
    fn argv_input_is_single_arg() {
        let p = argv_preset(&["grep", "{input}", "{files}"]);
        let got = expect_argv(
            p.expand(&names(&["a"]), &paths(&["/x/a"]), cwd(), "foo bar")
                .unwrap(),
        );
        assert_eq!(got, vec!["grep", "foo bar", "a"]);
    }

    #[test]
    fn argv_cwd_is_single_arg() {
        let p = argv_preset(&["cd", "{cwd}"]);
        let got = expect_argv(p.expand(&[], &[], cwd(), "").unwrap());
        assert_eq!(got, vec!["cd", "/cwd"]);
    }

    #[test]
    fn argv_cwd_embedded_in_arg() {
        let p = argv_preset(&["sh", "--rcfile={cwd}/.rc"]);
        let got = expect_argv(p.expand(&[], &[], cwd(), "").unwrap());
        assert_eq!(got, vec!["sh", "--rcfile=/cwd/.rc"]);
    }

    #[test]
    fn argv_embedded_placeholder_with_single_file_substitutes() {
        let p = argv_preset(&["mv", "{files}", "{files}.bak"]);
        let got = expect_argv(
            p.expand(&names(&["a"]), &paths(&["/x/a"]), cwd(), "")
                .unwrap(),
        );
        assert_eq!(got, vec!["mv", "a", "a.bak"]);
    }

    #[test]
    fn argv_embedded_placeholder_with_many_files_errors() {
        let p = argv_preset(&["mv", "{files}", "{paths}.bak"]);
        let Err(err) = p.expand(&names(&["a", "b"]), &paths(&["/x/a", "/x/b"]), cwd(), "") else {
            panic!("expected error")
        };
        assert!(err.contains("embedded"));
    }

    #[test]
    fn shell_quotes_filenames_with_spaces() {
        let p = shell_preset("ls {files}");
        let got = expect_shell(
            p.expand(&names(&["my file"]), &paths(&["/x/my file"]), cwd(), "")
                .unwrap(),
        );
        assert_eq!(got, "ls 'my file'");
    }

    #[test]
    fn shell_escapes_single_quotes_in_filenames() {
        let p = shell_preset("ls {files}");
        let got = expect_shell(
            p.expand(&names(&["it's"]), &paths(&["/x/it's"]), cwd(), "")
                .unwrap(),
        );
        assert_eq!(got, r"ls 'it'\''s'");
    }

    #[test]
    fn shell_quotes_input() {
        let p = shell_preset("grep {input} {files}");
        let got = expect_shell(
            p.expand(&names(&["a"]), &paths(&["/x/a"]), cwd(), "hello world")
                .unwrap(),
        );
        assert_eq!(got, "grep 'hello world' 'a'");
    }

    #[test]
    fn shell_quotes_cwd() {
        let p = shell_preset("ls {cwd}");
        let dir = Path::new("/path with space");
        let got = expect_shell(p.expand(&[], &[], dir, "").unwrap());
        assert_eq!(got, "ls '/path with space'");
    }

    #[test]
    fn needs_input_detects_placeholder() {
        assert!(argv_preset(&["grep", "{input}", "f"]).needs_input());
        assert!(shell_preset("echo {input}").needs_input());
        assert!(!argv_preset(&["ls", "{files}"]).needs_input());
    }

    #[test]
    fn references_files_detects_either_placeholder() {
        assert!(argv_preset(&["ls", "{files}"]).references_files());
        assert!(argv_preset(&["cat", "{paths}"]).references_files());
        assert!(!argv_preset(&["true"]).references_files());
        assert!(!shell_preset("echo hello").references_files());
        // {cwd} doesn't require a selection — runs even with empty file list.
        assert!(!argv_preset(&["pwd-print", "{cwd}"]).references_files());
    }
}
