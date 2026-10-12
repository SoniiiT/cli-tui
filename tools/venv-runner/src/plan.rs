//! Command plans: what to run to create a venv or to start the project in
//! one, plus a runner that executes them with the terminal's own stdio.

use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::project::{venv_bin_dir, venv_python, DependencySource};

/// One process to run. The runner gives it the real terminal, so output,
/// `input()` and debuggers behave exactly as in a normal shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, OsString)>,
    pub env_remove: Vec<&'static str>,
}

impl Step {
    fn new(program: &Path, args: &[&str], cwd: &Path) -> Self {
        Self {
            program: program.to_path_buf(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            cwd: cwd.to_path_buf(),
            env: Vec::new(),
            env_remove: Vec::new(),
        }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args).current_dir(&self.cwd);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        for key in &self.env_remove {
            command.env_remove(key);
        }
        command
    }

    /// Shell-style command line, relative to the project when possible.
    pub fn display(&self) -> String {
        let program = self
            .program
            .strip_prefix(&self.cwd)
            .unwrap_or(&self.program);
        let mut parts = vec![program.to_string_lossy().into_owned()];
        parts.extend(self.args.iter().cloned());
        shell_words::join(parts)
    }
}

pub struct CreateRequest<'a> {
    pub name: &'a str,
    pub interpreter: &'a Path,
    pub dependencies: Option<&'a DependencySource>,
}

/// `python -m venv <name>`, then optionally `<name>/bin/python -m pip install ...`.
pub fn plan_create(project: &Path, request: &CreateRequest<'_>) -> Vec<Step> {
    let mut steps = vec![Step::new(
        request.interpreter,
        &["-m", "venv", request.name],
        project,
    )];
    if let Some(source) = request.dependencies {
        let python = venv_python(&project.join(request.name));
        let mut step = Step::new(&python, &["-m", "pip", "install"], project);
        step.args.extend(source.pip_args());
        steps.push(step);
    }
    steps
}

/// Runs the venv's own interpreter with `run_line` as its arguments
/// (`main.py`, `main.py --debug`, `-m uvicorn app:app --reload`), with the
/// environment an activated venv would have.
pub fn plan_start(project: &Path, venv: &Path, run_line: &str) -> Result<Step, String> {
    let args = shell_words::split(run_line)
        .map_err(|error| format!("Cannot parse the run command: {error}."))?;
    if args.is_empty() {
        return Err("The run command is empty — press e to set one, e.g. main.py.".to_owned());
    }
    Ok(Step {
        program: venv_python(venv),
        args,
        cwd: project.to_path_buf(),
        env: activation_env(venv),
        env_remove: vec!["PYTHONHOME"],
    })
}

fn activation_env(venv: &Path) -> Vec<(String, OsString)> {
    let bin = venv_bin_dir(venv);
    let current = env::var_os("PATH");
    let path = match &current {
        Some(current) => {
            let mut dirs = vec![bin.clone()];
            dirs.extend(env::split_paths(current));
            env::join_paths(dirs).unwrap_or_else(|_| current.clone())
        }
        None => bin.into_os_string(),
    };
    vec![
        ("VIRTUAL_ENV".to_owned(), venv.as_os_str().to_owned()),
        ("PATH".to_owned(), path),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// Index of the first step that failed, `None` if all succeeded.
    pub failed_step: Option<usize>,
    pub summary: String,
}

/// Runs the steps one after another with inherited stdio and stops at the
/// first failure. Ctrl+C is left to the child for the duration.
pub fn run_steps(steps: &[Step], log: &mut dyn Write) -> RunOutcome {
    let _guard = InterruptGuard::install();
    for (index, step) in steps.iter().enumerate() {
        let _ = writeln!(log, "\n$ {}", step.display());
        let _ = log.flush();
        match step.command().status() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                return RunOutcome {
                    failed_step: Some(index),
                    summary: describe_status(status),
                }
            }
            Err(error) => {
                return RunOutcome {
                    failed_step: Some(index),
                    summary: format!("could not start {}: {error}", step.program.display()),
                }
            }
        }
    }
    RunOutcome {
        failed_step: None,
        summary: "exited with code 0".to_owned(),
    }
}

pub fn describe_status(status: ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exited with code {code}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return match signal {
                2 => "interrupted (SIGINT)".to_owned(),
                15 => "terminated (SIGTERM)".to_owned(),
                _ => format!("terminated by signal {signal}"),
            };
        }
    }
    "terminated".to_owned()
}

/// While a child owns the terminal, Ctrl+C reaches both the child and this
/// process (same foreground process group). A no-op handler keeps this process
/// alive without making the child ignore it: handlers are reset on `exec`,
/// whereas `SIG_IGN` would be inherited and break `KeyboardInterrupt`.
#[cfg(unix)]
struct InterruptGuard {
    previous: libc::sighandler_t,
}

#[cfg(unix)]
extern "C" fn ignore_interrupt(_signal: libc::c_int) {}

#[cfg(unix)]
impl InterruptGuard {
    fn install() -> Self {
        let handler: extern "C" fn(libc::c_int) = ignore_interrupt;
        // SAFETY: `signal` with a valid, async-signal-safe (empty) handler.
        let previous = unsafe { libc::signal(libc::SIGINT, handler as libc::sighandler_t) };
        Self { previous }
    }
}

#[cfg(unix)]
impl Drop for InterruptGuard {
    fn drop(&mut self) {
        // SAFETY: restores the handler that `install` replaced.
        unsafe {
            libc::signal(libc::SIGINT, self.previous);
        }
    }
}

/// Windows delivers Ctrl+C to the whole console group; not verified there.
#[cfg(not(unix))]
struct InterruptGuard;

#[cfg(not(unix))]
impl InterruptGuard {
    fn install() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn create_plan_makes_the_venv_then_installs_from_the_chosen_source() {
        let project = Path::new("/work/proj");
        let source = DependencySource::Requirements("requirements.txt".to_owned());

        let steps = plan_create(
            project,
            &CreateRequest {
                name: ".venv-abc",
                interpreter: Path::new("/opt/py/bin/python3.12"),
                dependencies: Some(&source),
            },
        );

        assert_eq!(steps.len(), 2);
        assert_eq!(
            steps[0].display(),
            "/opt/py/bin/python3.12 -m venv .venv-abc"
        );
        assert_eq!(steps[0].cwd, project);
        assert_eq!(
            steps[1].display(),
            ".venv-abc/bin/python -m pip install -r requirements.txt"
        );
    }

    #[test]
    fn create_plan_without_dependencies_is_a_single_step() {
        let steps = plan_create(
            Path::new("/work/proj"),
            &CreateRequest {
                name: ".venv",
                interpreter: Path::new("/opt/py/bin/python3"),
                dependencies: None,
            },
        );

        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn project_dependencies_install_editable() {
        let source = DependencySource::Project("pyproject.toml");
        let steps = plan_create(
            Path::new("/work/proj"),
            &CreateRequest {
                name: ".venv",
                interpreter: Path::new("/opt/py/bin/python3"),
                dependencies: Some(&source),
            },
        );

        assert_eq!(steps[1].display(), ".venv/bin/python -m pip install -e .");
    }

    #[test]
    fn start_plan_uses_the_venv_python_and_an_activated_environment() {
        let project = Path::new("/work/proj");
        let venv = project.join(".venv");

        let step = plan_start(project, &venv, "main.py --name 'Jane Doe'").unwrap();

        assert_eq!(step.program, venv_python(&venv));
        assert_eq!(step.args, ["main.py", "--name", "Jane Doe"]);
        assert_eq!(step.cwd, project);
        assert_eq!(step.display(), ".venv/bin/python main.py --name 'Jane Doe'");
        let virtual_env = step
            .env
            .iter()
            .find(|(key, _)| key == "VIRTUAL_ENV")
            .unwrap();
        assert_eq!(virtual_env.1, venv.as_os_str());
        let path = &step.env.iter().find(|(key, _)| key == "PATH").unwrap().1;
        assert_eq!(env::split_paths(path).next().unwrap(), venv_bin_dir(&venv));
        assert!(step.env_remove.contains(&"PYTHONHOME"));
    }

    #[test]
    fn start_plan_accepts_module_invocations_and_rejects_bad_input() {
        let project = Path::new("/work/proj");
        let venv = project.join(".venv");

        let step = plan_start(project, &venv, "-m uvicorn app:app --reload").unwrap();
        assert_eq!(step.args, ["-m", "uvicorn", "app:app", "--reload"]);

        assert!(plan_start(project, &venv, "   ").is_err());
        assert!(plan_start(project, &venv, "main.py 'unterminated").is_err());
    }

    fn any_interpreter() -> Option<PathBuf> {
        python::discover().into_iter().next().map(|i| i.path)
    }

    /// Creates a real venv from a real interpreter, installs a (comment-only,
    /// so offline) requirements file, and runs scripts inside it.
    #[test]
    fn creates_a_venv_and_runs_code_inside_it() {
        let Some(interpreter) = any_interpreter() else {
            return;
        };
        let project = tempdir().unwrap();
        let project_path = fs::canonicalize(project.path()).unwrap();
        fs::write(
            project_path.join("requirements.txt"),
            "# nothing to install\n",
        )
        .unwrap();
        let source = DependencySource::Requirements("requirements.txt".to_owned());

        let steps = plan_create(
            &project_path,
            &CreateRequest {
                name: ".venv-test",
                interpreter: &interpreter,
                dependencies: Some(&source),
            },
        );
        let outcome = run_steps(&steps, &mut Vec::new());

        assert_eq!(outcome.failed_step, None, "{}", outcome.summary);
        let venv = project_path.join(".venv-test");
        assert!(venv.join("pyvenv.cfg").is_file());
        assert!(venv_python(&venv).exists());

        fs::write(
            project_path.join("main.py"),
            "import os, sys\nprint(os.environ['VIRTUAL_ENV'])\nprint(sys.prefix)\nsys.exit(7)\n",
        )
        .unwrap();
        let step = plan_start(&project_path, &venv, "main.py").unwrap();
        let output = step.command().output().unwrap();

        assert_eq!(output.status.code(), Some(7));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<_> = stdout.lines().collect();
        assert_eq!(fs::canonicalize(lines[0]).unwrap(), venv);
        assert_eq!(fs::canonicalize(lines[1]).unwrap(), venv);
    }

    #[test]
    fn run_steps_stops_at_the_failing_step_and_reports_the_exit_code() {
        let Some(interpreter) = any_interpreter() else {
            return;
        };
        let cwd = tempdir().unwrap();
        let failing = Step::new(&interpreter, &["-c", "raise SystemExit(3)"], cwd.path());
        let never = Step::new(&interpreter, &["-c", "raise SystemExit(0)"], cwd.path());
        let mut log = Vec::new();

        let outcome = run_steps(&[failing, never], &mut log);

        assert_eq!(outcome.failed_step, Some(0));
        assert_eq!(outcome.summary, "exited with code 3");
        let log = String::from_utf8(log).unwrap();
        assert_eq!(
            log.matches("$ ").count(),
            1,
            "second step must not run: {log}"
        );
    }

    #[test]
    fn run_steps_reports_a_missing_program() {
        let cwd = tempdir().unwrap();
        let step = Step::new(Path::new("/definitely/not/python"), &["x.py"], cwd.path());

        let outcome = run_steps(&[step], &mut Vec::new());

        assert_eq!(outcome.failed_step, Some(0));
        assert!(
            outcome.summary.starts_with("could not start"),
            "{}",
            outcome.summary
        );
    }
}
