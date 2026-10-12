//! Looking at a project directory: which venvs exist, what would run, and
//! what could be installed into a new venv.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// A virtual environment found directly inside the project directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Venv {
    pub name: String,
    pub path: PathBuf,
    pub python_version: Option<String>,
    /// `false` when the interpreter inside the venv is missing or a dangling
    /// symlink, e.g. after the base Python was uninstalled.
    pub healthy: bool,
}

pub fn venv_bin_dir(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts")
    } else {
        venv.join("bin")
    }
}

pub fn venv_python(venv: &Path) -> PathBuf {
    venv_bin_dir(venv).join(if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    })
}

/// Finds venvs by their `pyvenv.cfg` marker, so any folder name works
/// (`.venv`, `venv`, `.venv-abc`, ...). Only direct children are inspected.
pub fn scan_venvs(project: &Path) -> Vec<Venv> {
    let Ok(entries) = fs::read_dir(project) else {
        return Vec::new();
    };
    let mut venvs: Vec<Venv> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let cfg = fs::read_to_string(path.join("pyvenv.cfg")).ok()?;
            Some(Venv {
                name: entry.file_name().to_string_lossy().into_owned(),
                healthy: venv_python(&path).exists(),
                python_version: parse_pyvenv_version(&cfg),
                path,
            })
        })
        .collect();
    venvs.sort_by(|a, b| {
        venv_rank(&a.name)
            .cmp(&venv_rank(&b.name))
            .then_with(|| a.name.cmp(&b.name))
    });
    venvs
}

fn venv_rank(name: &str) -> u8 {
    match name {
        ".venv" => 0,
        "venv" => 1,
        _ => 2,
    }
}

/// `python -m venv` writes `version = 3.12.4`; virtualenv and uv write
/// `version_info = 3.12.4.final.0` (and uv may omit `version`).
pub fn parse_pyvenv_version(cfg: &str) -> Option<String> {
    let mut fallback = None;
    for line in cfg.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "version" => return Some(value.to_owned()),
            "version_info" if fallback.is_none() => {
                fallback = Some(value.split('.').take(3).collect::<Vec<_>>().join("."));
            }
            _ => {}
        }
    }
    fallback
}

const MARKERS: [&str; 7] = [
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "setup.cfg",
    "Pipfile",
    ".python-version",
    "main.py",
];

/// Files that suggest a Python project, for the header line.
pub fn project_markers(project: &Path) -> Vec<&'static str> {
    MARKERS
        .iter()
        .copied()
        .filter(|marker| project.join(marker).exists())
        .collect()
}

const ENTRY_CANDIDATES: [&str; 6] = [
    "main.py",
    "app.py",
    "run.py",
    "src/main.py",
    "app/main.py",
    "__main__.py",
];

pub fn detect_entry(project: &Path) -> Option<String> {
    ENTRY_CANDIDATES
        .iter()
        .find(|candidate| project.join(candidate).is_file())
        .map(|candidate| (*candidate).to_owned())
}

/// Something that can be installed into a fresh venv.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Requirements(String),
    Project(&'static str),
}

impl DependencySource {
    pub fn label(&self) -> String {
        match self {
            Self::Requirements(file) => file.clone(),
            Self::Project(file) => format!("{file} (pip install -e .)"),
        }
    }

    /// Arguments that follow `python -m pip install`.
    pub fn pip_args(&self) -> Vec<String> {
        match self {
            Self::Requirements(file) => vec!["-r".to_owned(), file.clone()],
            Self::Project(_) => vec!["-e".to_owned(), ".".to_owned()],
        }
    }
}

/// `requirements*.txt` files (plain `requirements.txt` first), then an
/// installable `pyproject.toml` / `setup.py`.
pub fn dependency_sources(project: &Path) -> Vec<DependencySource> {
    let mut files: Vec<String> = fs::read_dir(project)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("requirements") && name.ends_with(".txt"))
        .collect();
    files.sort_by(|a, b| {
        (a != "requirements.txt")
            .cmp(&(b != "requirements.txt"))
            .then_with(|| a.cmp(b))
    });
    let mut sources: Vec<DependencySource> = files
        .into_iter()
        .map(DependencySource::Requirements)
        .collect();
    if has_installable_pyproject(project) {
        sources.push(DependencySource::Project("pyproject.toml"));
    } else if project.join("setup.py").is_file() {
        sources.push(DependencySource::Project("setup.py"));
    }
    sources
}

/// A pyproject that only configures tools (ruff, black, ...) cannot be
/// installed, so require a `[project]` or `[tool.poetry]` table.
fn has_installable_pyproject(project: &Path) -> bool {
    fs::read_to_string(project.join("pyproject.toml")).is_ok_and(|text| {
        text.lines()
            .map(str::trim)
            .any(|line| line == "[project]" || line == "[tool.poetry]")
    })
}

/// Raw `requires-python` value from `pyproject.toml`, shown as a hint only.
pub fn requires_python(project: &Path) -> Option<String> {
    let text = fs::read_to_string(project.join("pyproject.toml")).ok()?;
    text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("requires-python")?;
        let value = rest.trim_start().strip_prefix('=')?;
        Some(value.trim().trim_matches(['"', '\'']).to_owned())
    })
}

/// Version from `.python-version` (pyenv / uv): `3.12`, `3.12.4`, `cpython-3.12`.
pub fn python_version_hint(project: &Path) -> Option<String> {
    let text = fs::read_to_string(project.join(".python-version")).ok()?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))?;
    let token = line.split_whitespace().next()?;
    let start = token.find(|c: char| c.is_ascii_digit())?;
    if !matches!(&token[..start], "" | "cpython-") {
        return None;
    }
    let version: String = token[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let version = version.trim_end_matches('.');
    (!version.is_empty()).then(|| version.to_owned())
}

pub fn validate_venv_name(project: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name must not be empty.".to_owned());
    }
    let allowed = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !allowed || name == "." || name == ".." || name.starts_with('-') {
        return Err("Use only letters, digits, '.', '_' and '-' (no leading '-').".to_owned());
    }
    if project.join(name).exists() {
        return Err(format!("'{name}' already exists in this project."));
    }
    Ok(())
}

/// `.venv` if it is free, otherwise `.venv-2`, `.venv-3`, ...
pub fn suggest_venv_name(project: &Path) -> String {
    if !project.join(".venv").exists() {
        return ".venv".to_owned();
    }
    (2..)
        .map(|n| format!(".venv-{n}"))
        .find(|name| !project.join(name).exists())
        .expect("an unbounded range always yields a free name")
}

/// What deleting a venv would remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirStats {
    pub files: u64,
    pub bytes: u64,
}

/// Counts files and bytes below `path` without following symlinks, so a link
/// pointing at something big elsewhere is counted as the single entry it is.
pub fn dir_stats(path: &Path) -> DirStats {
    let mut stats = DirStats { files: 0, bytes: 0 };
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(entry.path());
            } else {
                stats.files += 1;
                stats.bytes += entry.metadata().map_or(0, |metadata| metadata.len());
            }
        }
    }
    stats
}

/// Where a venv folder that is a symlink points, `None` for a real folder.
pub fn symlink_target(path: &Path) -> Option<PathBuf> {
    fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.file_type().is_symlink())
        .and_then(|_| fs::read_link(path).ok())
}

/// Deletes the venv folder `name` directly inside `project`.
///
/// This is the one destructive operation of the tool, so it re-checks
/// everything itself instead of trusting the caller: `name` must be a single
/// plain path component (no `..`, no separators), and the folder must still be
/// a venv (contain `pyvenv.cfg`). A symlinked venv loses only the link, never
/// the folder it points to.
pub fn delete_venv(project: &Path, name: &str) -> Result<(), String> {
    let mut components = Path::new(name).components();
    if !matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    ) {
        return Err(format!("'{name}' is not a venv folder name."));
    }
    let path = project.join(name);
    if !path.join("pyvenv.cfg").is_file() {
        return Err(format!(
            "{name} is no longer a virtual environment (no pyvenv.cfg), so nothing was deleted."
        ));
    }
    // `remove_dir_all` does not follow a symlink at `path`; it removes the link.
    fs::remove_dir_all(&path).map_err(|error| format!("Could not delete {name}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_venv(project: &Path, name: &str, cfg: &str, with_python: bool) {
        let dir = project.join(name);
        fs::create_dir_all(venv_bin_dir(&dir)).unwrap();
        fs::write(dir.join("pyvenv.cfg"), cfg).unwrap();
        if with_python {
            fs::write(venv_python(&dir), "").unwrap();
        }
    }

    #[test]
    fn scan_finds_any_folder_with_pyvenv_cfg_and_ignores_the_rest() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv-abc", "version = 3.12.4\n", true);
        make_venv(project.path(), ".venv", "version = 3.11.9\n", true);
        fs::create_dir(project.path().join("src")).unwrap();
        fs::write(project.path().join("notes.txt"), "").unwrap();

        let venvs = scan_venvs(project.path());

        let names: Vec<_> = venvs.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, [".venv", ".venv-abc"]);
        assert_eq!(venvs[0].python_version.as_deref(), Some("3.11.9"));
        assert!(venvs.iter().all(|v| v.healthy));
    }

    #[test]
    fn scan_marks_a_venv_without_interpreter_as_broken() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv", "version = 3.12.4\n", false);

        let venvs = scan_venvs(project.path());

        assert_eq!(venvs.len(), 1);
        assert!(!venvs[0].healthy);
    }

    #[cfg(unix)]
    #[test]
    fn scan_marks_a_dangling_interpreter_symlink_as_broken() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv", "version = 3.12.4\n", false);
        let python = venv_python(&project.path().join(".venv"));
        std::os::unix::fs::symlink("/nonexistent/python3.12", &python).unwrap();

        assert!(!scan_venvs(project.path())[0].healthy);
    }

    #[test]
    fn scan_of_a_missing_directory_is_empty() {
        assert!(scan_venvs(Path::new("/definitely/not/here")).is_empty());
    }

    #[test]
    fn pyvenv_version_understands_venv_virtualenv_and_uv_formats() {
        assert_eq!(
            parse_pyvenv_version("home = /x\nversion = 3.12.4\n").as_deref(),
            Some("3.12.4")
        );
        assert_eq!(
            parse_pyvenv_version("home = /x\nversion_info = 3.11.9.final.0\n").as_deref(),
            Some("3.11.9")
        );
        assert_eq!(parse_pyvenv_version("home = /x\n"), None);
    }

    #[test]
    fn entry_detection_prefers_main_py_and_falls_back() {
        let project = tempdir().unwrap();
        assert_eq!(detect_entry(project.path()), None);

        fs::create_dir(project.path().join("src")).unwrap();
        fs::write(project.path().join("src/main.py"), "").unwrap();
        assert_eq!(detect_entry(project.path()).as_deref(), Some("src/main.py"));

        fs::write(project.path().join("app.py"), "").unwrap();
        assert_eq!(detect_entry(project.path()).as_deref(), Some("app.py"));

        fs::write(project.path().join("main.py"), "").unwrap();
        assert_eq!(detect_entry(project.path()).as_deref(), Some("main.py"));
    }

    #[test]
    fn dependency_sources_list_requirements_first_then_an_installable_project() {
        let project = tempdir().unwrap();
        fs::write(project.path().join("requirements-dev.txt"), "").unwrap();
        fs::write(project.path().join("requirements.txt"), "").unwrap();
        fs::write(
            project.path().join("pyproject.toml"),
            "[project]\nname='x'\n",
        )
        .unwrap();

        let sources = dependency_sources(project.path());

        assert_eq!(
            sources,
            [
                DependencySource::Requirements("requirements.txt".to_owned()),
                DependencySource::Requirements("requirements-dev.txt".to_owned()),
                DependencySource::Project("pyproject.toml"),
            ]
        );
    }

    #[test]
    fn a_tool_only_pyproject_is_not_offered_for_installation() {
        let project = tempdir().unwrap();
        fs::write(
            project.path().join("pyproject.toml"),
            "[tool.ruff]\nline-length = 100\n",
        )
        .unwrap();

        assert!(dependency_sources(project.path()).is_empty());
    }

    #[test]
    fn requires_python_and_version_hint_are_read_from_the_project() {
        let project = tempdir().unwrap();
        fs::write(
            project.path().join("pyproject.toml"),
            "[project]\nrequires-python = \">=3.11\"\n",
        )
        .unwrap();
        fs::write(
            project.path().join(".python-version"),
            "# comment\n3.12.4\n",
        )
        .unwrap();

        assert_eq!(requires_python(project.path()).as_deref(), Some(">=3.11"));
        assert_eq!(
            python_version_hint(project.path()).as_deref(),
            Some("3.12.4")
        );
    }

    #[test]
    fn version_hint_ignores_non_cpython_and_unversioned_entries() {
        let project = tempdir().unwrap();
        for (content, expected) in [
            ("cpython-3.12\n", Some("3.12")),
            ("system\n", None),
            ("pypy3.10\n", None),
            ("3.13.\n", Some("3.13")),
        ] {
            fs::write(project.path().join(".python-version"), content).unwrap();
            assert_eq!(
                python_version_hint(project.path()).as_deref(),
                expected,
                "{content:?}"
            );
        }
    }

    #[test]
    fn venv_names_are_validated_against_the_project() {
        let project = tempdir().unwrap();
        fs::create_dir(project.path().join(".venv")).unwrap();

        assert!(validate_venv_name(project.path(), ".venv-abc").is_ok());
        assert!(validate_venv_name(project.path(), "").is_err());
        assert!(validate_venv_name(project.path(), "..").is_err());
        assert!(validate_venv_name(project.path(), "a/b").is_err());
        assert!(validate_venv_name(project.path(), "-x").is_err());
        assert!(validate_venv_name(project.path(), "my venv").is_err());
        assert!(validate_venv_name(project.path(), ".venv")
            .unwrap_err()
            .contains("already exists"));
    }

    #[test]
    fn suggested_name_skips_taken_ones() {
        let project = tempdir().unwrap();
        assert_eq!(suggest_venv_name(project.path()), ".venv");
        fs::create_dir(project.path().join(".venv")).unwrap();
        assert_eq!(suggest_venv_name(project.path()), ".venv-2");
        fs::create_dir(project.path().join(".venv-2")).unwrap();
        assert_eq!(suggest_venv_name(project.path()), ".venv-3");
    }

    #[test]
    fn deleting_a_venv_removes_it_and_nothing_else_in_the_project() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv-abc", "version = 3.12.4\n", true);
        make_venv(project.path(), ".venv", "version = 3.12.4\n", true);
        fs::write(project.path().join("main.py"), "print('x')").unwrap();
        fs::create_dir(project.path().join("src")).unwrap();
        fs::write(project.path().join("src/app.py"), "").unwrap();

        delete_venv(project.path(), ".venv-abc").unwrap();

        assert!(!project.path().join(".venv-abc").exists());
        assert!(
            project.path().join(".venv/pyvenv.cfg").is_file(),
            "sibling venv"
        );
        assert!(project.path().join("main.py").is_file());
        assert!(project.path().join("src/app.py").is_file());
    }

    #[test]
    fn deleting_a_broken_venv_works_too() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv", "version = 3.9.1\n", false);

        delete_venv(project.path(), ".venv").unwrap();

        assert!(!project.path().join(".venv").exists());
    }

    #[test]
    fn deleting_refuses_folders_that_are_not_venvs() {
        let project = tempdir().unwrap();
        fs::create_dir(project.path().join("src")).unwrap();
        fs::write(project.path().join("src/app.py"), "").unwrap();

        let error = delete_venv(project.path(), "src").unwrap_err();

        assert!(error.contains("no longer a virtual environment"), "{error}");
        assert!(project.path().join("src/app.py").is_file());
        assert!(delete_venv(project.path(), "missing").is_err());
    }

    #[test]
    fn deleting_refuses_names_that_could_leave_the_project() {
        let outer = tempdir().unwrap();
        let project = outer.path().join("proj");
        fs::create_dir(&project).unwrap();
        // The parent of the project is itself a venv-looking folder, and so is a
        // sibling: neither may be reachable through the name.
        fs::write(outer.path().join("pyvenv.cfg"), "").unwrap();
        make_venv(outer.path(), "sibling", "version = 3.12.4\n", true);
        fs::write(project.join("pyvenv.cfg"), "").unwrap();

        for name in ["..", ".", "", "../sibling", "sub/.venv", "/etc"] {
            assert!(
                delete_venv(&project, name).is_err(),
                "{name:?} must be refused"
            );
        }
        let absolute = outer.path().join("sibling");
        assert!(delete_venv(&project, absolute.to_str().unwrap()).is_err());

        assert!(outer.path().join("pyvenv.cfg").is_file());
        assert!(outer.path().join("sibling/pyvenv.cfg").is_file());
        assert!(project.join("pyvenv.cfg").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn deleting_a_symlinked_venv_removes_only_the_link() {
        let project = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        make_venv(elsewhere.path(), "shared-venv", "version = 3.12.4\n", true);
        let target = elsewhere.path().join("shared-venv");
        std::os::unix::fs::symlink(&target, project.path().join(".venv")).unwrap();
        assert_eq!(
            symlink_target(&project.path().join(".venv")),
            Some(target.clone())
        );

        delete_venv(project.path(), ".venv").unwrap();

        assert!(!project.path().join(".venv").exists());
        assert!(
            fs::symlink_metadata(project.path().join(".venv")).is_err(),
            "link is gone"
        );
        assert!(
            target.join("pyvenv.cfg").is_file(),
            "the real venv must survive"
        );
        assert!(venv_python(&target).exists());
    }

    #[test]
    fn a_real_folder_has_no_symlink_target() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv", "version = 3.12.4\n", true);

        assert_eq!(symlink_target(&project.path().join(".venv")), None);
        assert_eq!(symlink_target(&project.path().join("missing")), None);
    }

    #[test]
    fn dir_stats_counts_nested_files_and_bytes() {
        let project = tempdir().unwrap();
        let root = project.path().join(".venv");
        fs::create_dir_all(root.join("lib/site-packages/pkg")).unwrap();
        fs::write(root.join("pyvenv.cfg"), "12345").unwrap();
        fs::write(root.join("lib/a.py"), "123").unwrap();
        fs::write(root.join("lib/site-packages/pkg/b.py"), "1234567").unwrap();

        assert_eq!(
            dir_stats(&root),
            DirStats {
                files: 3,
                bytes: 15
            }
        );
        assert_eq!(
            dir_stats(&project.path().join("missing")),
            DirStats { files: 0, bytes: 0 }
        );
    }

    #[cfg(unix)]
    #[test]
    fn dir_stats_does_not_follow_symlinks() {
        let project = tempdir().unwrap();
        let big = tempdir().unwrap();
        fs::write(big.path().join("huge.bin"), vec![0_u8; 10_000]).unwrap();
        let root = project.path().join(".venv");
        fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink(big.path(), root.join("link-to-dir")).unwrap();

        let stats = dir_stats(&root);

        assert_eq!(stats.files, 1, "the link counts as one entry");
        assert!(stats.bytes < 10_000, "linked content must not be counted");
    }
}
