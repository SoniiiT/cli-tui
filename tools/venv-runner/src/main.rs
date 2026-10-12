mod app;
mod ui;

use std::io::{self, IsTerminal};
use std::panic;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use venv_runner::plan::{run_steps, RunOutcome};
use venv_runner::python;

use crate::app::{Action, App, Interpreters};

const HELP: &str = "\
venv-runner — find or create a Python virtual environment and run your project in it

Usage: venv-runner [PROJECT_DIR]

Opens a fullscreen TUI for PROJECT_DIR (default: the current directory).
  - If a venv exists (any folder containing pyvenv.cfg), start the project in it.
  - If not, create one: choose the folder name, the Python version and
    which dependencies to install.
Starting leaves the TUI and runs `<venv>/bin/python <run command>` directly in
your terminal, so output, input() and debuggers work as usual.

Options:
  -h, --help       Show this help
  -V, --version    Show the version";

enum Cli {
    Run(PathBuf),
    Exit,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Cli> {
    let mut project = None;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(Cli::Exit);
            }
            "-V" | "--version" => {
                println!("venv-runner {}", env!("CARGO_PKG_VERSION"));
                return Ok(Cli::Exit);
            }
            flag if flag.starts_with('-') => bail!("unknown option '{flag}' (see --help)"),
            _ if project.is_some() => bail!("only one project directory can be given (see --help)"),
            path => project = Some(PathBuf::from(path)),
        }
    }
    Ok(Cli::Run(project.unwrap_or_else(|| PathBuf::from("."))))
}

fn main() -> Result<()> {
    let Cli::Run(project) = parse_args(std::env::args().skip(1))? else {
        return Ok(());
    };
    let project = project
        .canonicalize()
        .with_context(|| format!("cannot open {}", project.display()))?;
    if !project.is_dir() {
        bail!("{} is not a directory", project.display());
    }
    if !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        bail!("venv-runner needs an interactive terminal");
    }

    // Probing interpreters starts real processes; do it off the UI thread.
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(python::discover());
    });

    install_panic_hook();
    let mut tui = Tui::enter()?;
    let mut app = App::new(project, Interpreters::Loading(receiver));
    let result = run_app(&mut tui, &mut app);
    let left = tui.leave();
    result.and(left)
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

/// Owns the terminal UI and restores the terminal even on early return.
struct Tui {
    terminal: Term,
    active: bool,
}

impl Tui {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("enabling terminal raw mode")?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error).context("entering alternate screen");
        }
        let mut terminal = match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = disable_raw_mode();
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
                return Err(error).context("creating terminal interface");
            }
        };
        terminal.hide_cursor().context("hiding terminal cursor")?;
        Ok(Self {
            terminal,
            active: true,
        })
    }

    fn leave(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let raw = disable_raw_mode().context("restoring terminal mode");
        let screen = execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("leaving alternate screen");
        let cursor = self
            .terminal
            .show_cursor()
            .context("restoring terminal cursor");
        raw.and(screen).and(cursor)
    }

    fn resume(&mut self) -> Result<()> {
        enable_raw_mode().context("enabling terminal raw mode")?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)
            .context("entering alternate screen")?;
        self.active = true;
        self.terminal
            .hide_cursor()
            .context("hiding terminal cursor")?;
        // The screen content was replaced while a program owned the terminal.
        self.terminal.clear().context("clearing terminal")
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

fn install_panic_hook() {
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default(info);
    }));
}

fn run_app(tui: &mut Tui, app: &mut App) -> Result<()> {
    // Redraw only when something changed; an idle TUI should not keep writing
    // to the terminal (it adds up over SSH and makes the screen flicker).
    let mut needs_draw = true;
    loop {
        let was_loading = app.python_loading();
        app.poll_interpreters();
        needs_draw |= was_loading && !app.python_loading();
        if needs_draw {
            tui.terminal.draw(|frame| ui::draw(frame, app))?;
            needs_draw = false;
        }
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
            // Any event, including a resize, can change what is on screen.
            needs_draw = true;
        }
        if let Some(action) = app.pending.take() {
            let (outcome, quit) = run_in_terminal(tui, &action)?;
            app.finish(&action, &outcome, quit);
            needs_draw = true;
        }
        if app.quit {
            return Ok(());
        }
    }
}

/// Hands the real terminal to the child processes: leaves the alternate
/// screen, runs the steps with inherited stdio (so `print`, `input()`, pdb and
/// Ctrl+C behave as in a normal shell), then waits for a key before the TUI
/// takes the screen back, so the output stays readable.
fn run_in_terminal(tui: &mut Tui, action: &Action) -> Result<(RunOutcome, bool)> {
    // Keystrokes typed while the TUI was busy must not leak into the child.
    while event::poll(Duration::ZERO)? {
        let _ = event::read()?;
    }
    tui.leave()?;

    let outcome = run_steps(action.steps(), &mut io::stdout());
    let (mark, verdict) = if outcome.failed_step.is_none() {
        ("✔", "finished".to_owned())
    } else {
        ("✘", outcome.summary.clone())
    };
    println!("\n── {mark} {verdict} ──");
    println!("Press Enter to return to venv-runner, or q to quit.");

    let quit = wait_for_return()?;
    if !quit {
        tui.resume()?;
    }
    Ok((outcome, quit))
}

/// `true` if the user asked to quit instead of returning to the TUI.
fn wait_for_return() -> Result<bool> {
    enable_raw_mode().context("enabling terminal raw mode")?;
    let answer = loop {
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Enter => break false,
            KeyCode::Char('q') | KeyCode::Esc => break true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break true,
            _ => {}
        }
    };
    disable_raw_mode().context("restoring terminal mode")?;
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn no_argument_means_the_current_directory() {
        let Cli::Run(path) = parse_args(args(&[])).unwrap() else {
            panic!("expected to run");
        };
        assert_eq!(path, PathBuf::from("."));
    }

    #[test]
    fn a_single_path_is_accepted() {
        let Cli::Run(path) = parse_args(args(&["../proj"])).unwrap() else {
            panic!("expected to run");
        };
        assert_eq!(path, PathBuf::from("../proj"));
    }

    #[test]
    fn help_and_version_exit_without_running() {
        assert!(matches!(parse_args(args(&["--help"])).unwrap(), Cli::Exit));
        assert!(matches!(parse_args(args(&["-V"])).unwrap(), Cli::Exit));
    }

    #[test]
    fn unknown_flags_and_extra_paths_are_errors() {
        assert!(parse_args(args(&["--nope"])).is_err());
        assert!(parse_args(args(&["a", "b"])).is_err());
    }
}
