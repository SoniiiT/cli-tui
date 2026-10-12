//! Finding Python interpreters a new venv can be created from.
//!
//! Candidates come from `PATH`, pyenv's version directory and uv's managed
//! Pythons. Every candidate is probed by actually running it: pyenv shims for
//! versions that are not active, Debian's Python without `ensurepip`, and
//! interpreters that live inside another venv are all dropped this way.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpreter {
    pub version: String,
    pub path: PathBuf,
    pub source: &'static str,
    /// What plain `python3` resolves to in the current environment.
    pub is_default: bool,
}

impl Interpreter {
    pub fn label(&self) -> String {
        format!("{} · {}", self.version, self.source)
    }
}

const PROBE_SCRIPT: &str = "import sys, venv, ensurepip\n\
if sys.prefix != sys.base_prefix:\n    raise SystemExit(3)\n\
print('.'.join(map(str, sys.version_info[:3])))\n\
print(sys.executable)";

#[derive(Debug, PartialEq, Eq)]
pub struct Probe {
    pub version: String,
    pub executable: PathBuf,
}

/// Runs `path` and reports its version and real executable, or `None` if it
/// cannot create venvs.
pub fn probe(path: &Path) -> Option<Probe> {
    let output = Command::new(path)
        .args(["-c", PROBE_SCRIPT])
        .env_remove("VIRTUAL_ENV")
        .env_remove("PYTHONHOME")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_probe_output(&String::from_utf8_lossy(&output.stdout))
}

/// The last two non-empty lines are version and executable; anything printed
/// before them (e.g. by a `sitecustomize`) is ignored.
pub fn parse_probe_output(text: &str) -> Option<Probe> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let [.., version, executable] = lines.as_slice() else {
        return None;
    };
    parse_version(version)?;
    Some(Probe {
        version: (*version).to_owned(),
        executable: PathBuf::from(executable),
    })
}

pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = match parts.next() {
        Some(part) => part.parse().ok()?,
        None => 0,
    };
    Some((major, minor, patch))
}

#[derive(Debug)]
struct Candidate {
    path: PathBuf,
    on_path: bool,
}

fn is_python_name(name: &str) -> bool {
    let name = name.strip_suffix(".exe").unwrap_or(name);
    name == "python"
        || name == "python3"
        || name
            .strip_prefix("python3.")
            .is_some_and(|minor| !minor.is_empty() && minor.chars().all(|c| c.is_ascii_digit()))
}

/// Whether `path` is exactly the command `name` (`python3`, not `python3.12`).
/// `Path::file_stem` would be wrong here: it treats the `.12` in `python3.12`
/// as an extension and returns `python3`.
fn is_command(path: &Path, name: &str) -> bool {
    path.file_name()
        .and_then(|file| file.to_str())
        .map(|file| file.strip_suffix(".exe").unwrap_or(file))
        == Some(name)
}

/// On macOS `/usr/bin/python3` is a stub that opens the Xcode Command Line
/// Tools installer when they are missing; never run it in that case.
#[cfg(target_os = "macos")]
fn is_runnable(path: &Path) -> bool {
    !path.starts_with("/usr/bin")
        || Command::new("xcode-select")
            .arg("-p")
            .output()
            .is_ok_and(|output| output.status.success())
}

#[cfg(not(target_os = "macos"))]
fn is_runnable(_path: &Path) -> bool {
    true
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn sorted_dir(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    paths
}

fn candidates() -> Vec<Candidate> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut push = |path: PathBuf, on_path: bool| {
        if seen.insert(path.clone()) {
            found.push(Candidate { path, on_path });
        }
    };

    for dir in env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default()
    {
        for path in sorted_dir(&dir) {
            let named_like_python = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_python_name);
            if named_like_python && path.is_file() && is_runnable(&path) {
                push(path, true);
            }
        }
    }

    // Versions that pyenv/uv installed but that are not active are only
    // reachable through their own directories.
    let pyenv = env::var_os("PYENV_ROOT")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".pyenv")))
        .map(|root| root.join("versions"));
    let uv = env::var_os("UV_PYTHON_INSTALL_DIR")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".local/share/uv/python")));
    for root in [pyenv, uv].into_iter().flatten() {
        for version_dir in sorted_dir(&root) {
            let bin = version_dir.join("bin");
            if let Some(python) = ["python3", "python"]
                .iter()
                .map(|name| bin.join(name))
                .find(|path| path.is_file())
            {
                push(python, false);
            }
        }
    }
    found
}

fn source_of(path: &Path) -> &'static str {
    let text = path.to_string_lossy();
    if text.contains("/.pyenv/") {
        "pyenv"
    } else if text.contains("/uv/python") {
        "uv"
    } else if text.starts_with("/opt/homebrew/")
        || text.contains("/Cellar/")
        || text.contains("linuxbrew")
    {
        "Homebrew"
    } else if text.starts_with("/usr/bin/") {
        "system"
    } else {
        "PATH"
    }
}

/// All usable interpreters, newest version first. Probes run in parallel
/// because each one starts a real Python process.
pub fn discover() -> Vec<Interpreter> {
    let candidates = candidates();
    let probes: Vec<Option<Probe>> = thread::scope(|scope| {
        let handles: Vec<_> = candidates
            .iter()
            .map(|candidate| scope.spawn(|| probe(&candidate.path)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().ok().flatten())
            .collect()
    });

    let probed: Vec<(Candidate, Probe, PathBuf)> = candidates
        .into_iter()
        .zip(probes)
        .filter_map(|(candidate, probe)| {
            let probe = probe?;
            let identity =
                fs::canonicalize(&probe.executable).unwrap_or_else(|_| probe.executable.clone());
            Some((candidate, probe, identity))
        })
        .collect();

    let default_identity = probed
        .iter()
        .find(|(candidate, _, _)| candidate.on_path && is_command(&candidate.path, "python3"))
        .or_else(|| {
            probed.iter().find(|(candidate, _, _)| {
                candidate.on_path && is_command(&candidate.path, "python")
            })
        })
        .map(|(_, _, identity)| identity.clone());

    let mut seen = HashSet::new();
    let mut interpreters: Vec<Interpreter> = probed
        .into_iter()
        .filter(|(_, _, identity)| seen.insert(identity.clone()))
        .map(|(candidate, probe, identity)| {
            // A shim only forwards to the real interpreter; create venvs from
            // the real one so the venv does not depend on shim configuration.
            let path = if candidate
                .path
                .components()
                .any(|c| c.as_os_str() == "shims")
            {
                probe.executable
            } else {
                candidate.path
            };
            Interpreter {
                version: probe.version,
                source: source_of(&path),
                is_default: default_identity.as_ref() == Some(&identity),
                path,
            }
        })
        .collect();
    interpreters.sort_by_key(|i| std::cmp::Reverse(parse_version(&i.version)));
    interpreters
}

/// Index to preselect: the `.python-version` match, else what `python3`
/// resolves to, else the newest.
pub fn preselect(interpreters: &[Interpreter], hint: Option<&str>) -> usize {
    hint.and_then(|hint| {
        interpreters.iter().position(|i| {
            i.version == hint
                || i.version
                    .strip_prefix(hint)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
    })
    .or_else(|| interpreters.iter().position(|i| i.is_default))
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpreter(version: &str, is_default: bool) -> Interpreter {
        Interpreter {
            version: version.to_owned(),
            path: PathBuf::from(format!("/py/{version}/bin/python3")),
            source: "PATH",
            is_default,
        }
    }

    #[test]
    fn probe_output_ignores_noise_before_the_two_result_lines() {
        let probe =
            parse_probe_output("hello from sitecustomize\n3.12.4\n/opt/py/bin/python3.12\n")
                .unwrap();

        assert_eq!(probe.version, "3.12.4");
        assert_eq!(probe.executable, PathBuf::from("/opt/py/bin/python3.12"));
        assert_eq!(parse_probe_output("3.12.4\n"), None);
        assert_eq!(parse_probe_output("not-a-version\n/bin/python\n"), None);
    }

    #[test]
    fn versions_parse_and_order_numerically() {
        assert_eq!(parse_version("3.9.6"), Some((3, 9, 6)));
        assert_eq!(parse_version("3.12"), Some((3, 12, 0)));
        assert!(parse_version("3.12.0") > parse_version("3.9.6"));
        assert_eq!(parse_version("x.y"), None);
    }

    #[test]
    fn only_python_executables_are_candidates() {
        for name in ["python", "python3", "python3.12", "python3.12.exe"] {
            assert!(is_python_name(name), "{name}");
        }
        for name in [
            "python3-config",
            "python3.12-config",
            "pythonw",
            "pip3",
            "python2",
        ] {
            assert!(!is_python_name(name), "{name}");
        }
    }

    #[test]
    fn only_the_bare_command_counts_as_the_default_python() {
        assert!(is_command(Path::new("/usr/bin/python3"), "python3"));
        assert!(is_command(Path::new("C:/Python/python3.exe"), "python3"));
        assert!(is_command(Path::new("/usr/bin/python"), "python"));
        assert!(
            !is_command(Path::new("/usr/bin/python3.12"), "python3"),
            "python3.12 must not be mistaken for python3"
        );
        assert!(!is_command(Path::new("/usr/bin/python3"), "python"));
    }

    #[test]
    fn preselect_prefers_the_version_hint_then_the_default_then_the_newest() {
        let list = [
            interpreter("3.14.7", false),
            interpreter("3.12.15", true),
            interpreter("3.11.16", false),
        ];

        assert_eq!(preselect(&list, Some("3.11")), 2);
        assert_eq!(preselect(&list, Some("3.12.15")), 1);
        assert_eq!(
            preselect(&list, Some("3.1")),
            1,
            "3.1 must not match 3.11 or 3.14"
        );
        assert_eq!(
            preselect(&list, Some("2.7")),
            1,
            "unknown hint falls back to the default"
        );
        assert_eq!(preselect(&list, None), 1);
        assert_eq!(preselect(&[interpreter("3.14.7", false)], None), 0);
        assert_eq!(preselect(&[], None), 0);
    }

    #[test]
    fn source_labels_follow_the_install_location() {
        assert_eq!(
            source_of(Path::new("/Users/me/.pyenv/versions/3.12.14/bin/python3")),
            "pyenv"
        );
        assert_eq!(
            source_of(Path::new(
                "/Users/me/.local/share/uv/python/cpython-3.12/bin/python3"
            )),
            "uv"
        );
        assert_eq!(
            source_of(Path::new("/opt/homebrew/bin/python3.12")),
            "Homebrew"
        );
        assert_eq!(source_of(Path::new("/usr/bin/python3")), "system");
        assert_eq!(source_of(Path::new("/srv/python/bin/python3")), "PATH");
    }

    /// Runs the real interpreter of whatever machine the tests are on.
    #[test]
    fn discovery_finds_the_python_that_probing_accepts() {
        let Some(python) = Command::new("python3")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())
        else {
            return;
        };
        drop(python);

        let found = discover();

        assert!(
            !found.is_empty(),
            "python3 runs, so discovery must find something"
        );
        for interpreter in &found {
            assert!(parse_version(&interpreter.version).is_some());
            assert!(interpreter.path.is_absolute(), "{:?}", interpreter.path);
        }
        let versions: Vec<_> = found.iter().map(|i| parse_version(&i.version)).collect();
        assert!(
            versions.windows(2).all(|pair| pair[0] >= pair[1]),
            "newest first"
        );
        let unique: HashSet<_> = found.iter().map(|i| &i.path).collect();
        assert_eq!(unique.len(), found.len());

        // The default must be what a shell resolves `python3` to (when that
        // one can create venvs at all).
        if let Some(shell_python) = probe(Path::new("python3")) {
            let defaults: Vec<_> = found.iter().filter(|i| i.is_default).collect();
            assert_eq!(defaults.len(), 1, "{found:?}");
            assert_eq!(
                fs::canonicalize(&defaults[0].path).unwrap(),
                fs::canonicalize(&shell_python.executable).unwrap(),
                "default is not what `python3` runs: {found:?}"
            );
        }
    }
}
