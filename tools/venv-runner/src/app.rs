//! TUI state and key handling. Nothing here touches the terminal: a key press
//! either changes state or leaves an [`Action`] in `pending` for the main loop
//! to run with the real terminal, which keeps this testable without a tty.

use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use venv_runner::plan::{plan_create, plan_start, CreateRequest, RunOutcome, Step};
use venv_runner::project::{
    delete_venv, dependency_sources, detect_entry, dir_stats, project_markers, python_version_hint,
    requires_python, scan_venvs, suggest_venv_name, symlink_target, validate_venv_name,
    DependencySource, DirStats, Venv,
};
use venv_runner::python::{preselect, Interpreter};

#[derive(Debug, Clone, Default)]
pub struct TextInput {
    pub value: String,
    pub cursor: usize,
}

impl TextInput {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
            cursor: value.chars().count(),
        }
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }

    fn byte_index(&self, chars: usize) -> usize {
        self.value
            .char_indices()
            .nth(chars)
            .map_or(self.value.len(), |(index, _)| index)
    }

    /// Returns whether the key was an editing key.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.value.clear();
                self.cursor = 0;
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.value.insert(self.byte_index(self.cursor), ch);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.value.remove(self.byte_index(self.cursor));
            }
            KeyCode::Delete if self.cursor < self.len() => {
                self.value.remove(self.byte_index(self.cursor));
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.len()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.len(),
            _ => return false,
        }
        true
    }

    pub fn display(&self) -> String {
        let at = self.byte_index(self.cursor);
        format!("{}▏{}", &self.value[..at], &self.value[at..])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Python,
    Dependencies,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Self::Name => Self::Python,
            Self::Python => Self::Dependencies,
            Self::Dependencies => Self::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Name => Self::Dependencies,
            Self::Python => Self::Name,
            Self::Dependencies => Self::Python,
        }
    }
}

fn step_index(current: usize, len: usize, forward: bool) -> usize {
    match (len, forward) {
        (0, _) => 0,
        (_, true) => (current + 1) % len,
        (_, false) => (current + len - 1) % len,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormEvent {
    None,
    Cancel,
    Submit,
}

#[derive(Debug, Clone)]
pub struct CreateForm {
    pub name: TextInput,
    pub field: Field,
    /// `None` until the user picks one; then the preselected interpreter
    /// (which may only be known after background discovery finishes) applies.
    pub python_choice: Option<usize>,
    pub dependency_options: Vec<Option<DependencySource>>,
    pub dependency_choice: usize,
    pub error: Option<String>,
}

impl CreateForm {
    fn new(project: &Path) -> Self {
        let mut dependency_options: Vec<Option<DependencySource>> =
            dependency_sources(project).into_iter().map(Some).collect();
        dependency_options.push(None);
        Self {
            name: TextInput::new(&suggest_venv_name(project)),
            field: Field::Name,
            python_choice: None,
            dependency_options,
            dependency_choice: 0,
            error: None,
        }
    }

    pub fn dependencies(&self) -> Option<&DependencySource> {
        self.dependency_options[self.dependency_choice].as_ref()
    }

    fn handle_key(
        &mut self,
        key: KeyEvent,
        python_count: usize,
        python_default: usize,
    ) -> FormEvent {
        match key.code {
            KeyCode::Esc => return FormEvent::Cancel,
            KeyCode::Enter => return FormEvent::Submit,
            _ => {}
        }
        self.error = None;
        match key.code {
            KeyCode::Tab | KeyCode::Down => self.field = self.field.next(),
            KeyCode::BackTab | KeyCode::Up => self.field = self.field.prev(),
            KeyCode::Left | KeyCode::Right if self.field != Field::Name => {
                let forward = key.code == KeyCode::Right;
                if self.field == Field::Python {
                    let current = self.python_choice.unwrap_or(python_default);
                    if python_count > 0 {
                        self.python_choice = Some(step_index(current, python_count, forward));
                    }
                } else {
                    self.dependency_choice = step_index(
                        self.dependency_choice,
                        self.dependency_options.len(),
                        forward,
                    );
                }
            }
            _ if self.field == Field::Name => {
                self.name.handle_key(key);
            }
            _ => {}
        }
        FormEvent::None
    }
}

/// Everything the delete dialog shows, captured when it opens.
#[derive(Debug, Clone)]
pub struct DeleteConfirm {
    pub name: String,
    pub path: PathBuf,
    pub python_version: Option<String>,
    pub healthy: bool,
    /// `None` for a symlinked venv: only the link goes, so its size is moot.
    pub stats: Option<DirStats>,
    pub symlink_target: Option<PathBuf>,
}

pub enum Screen {
    List,
    Create(CreateForm),
    EditRun(TextInput),
    ConfirmDelete(DeleteConfirm),
}

/// Work that needs the real terminal.
pub enum Action {
    Create { name: String, steps: Vec<Step> },
    Start { step: Step },
}

impl Action {
    pub fn steps(&self) -> &[Step] {
        match self {
            Self::Create { steps, .. } => steps,
            Self::Start { step } => std::slice::from_ref(step),
        }
    }
}

pub enum Interpreters {
    Loading(Receiver<Vec<Interpreter>>),
    Ready(Vec<Interpreter>),
}

pub struct App {
    pub project: PathBuf,
    pub venvs: Vec<Venv>,
    /// Index into the list rows: the venvs, then the "new venv" row.
    pub selected: usize,
    pub run_line: String,
    pub screen: Screen,
    pub interpreters: Interpreters,
    pub python_hint: Option<String>,
    pub requires_python: Option<String>,
    pub markers: Vec<&'static str>,
    pub status: Option<String>,
    pub pending: Option<Action>,
    pub quit: bool,
}

impl App {
    pub fn new(project: PathBuf, interpreters: Interpreters) -> Self {
        let venvs = scan_venvs(&project);
        let selected = venvs.iter().position(|venv| venv.healthy).unwrap_or(0);
        // Nothing to start yet: go straight to creating one.
        let screen = if venvs.is_empty() {
            Screen::Create(CreateForm::new(&project))
        } else {
            Screen::List
        };
        Self {
            run_line: detect_entry(&project).unwrap_or_else(|| "main.py".to_owned()),
            python_hint: python_version_hint(&project),
            requires_python: requires_python(&project),
            markers: project_markers(&project),
            venvs,
            selected,
            screen,
            interpreters,
            status: None,
            pending: None,
            quit: false,
            project,
        }
    }

    pub fn python_list(&self) -> &[Interpreter] {
        match &self.interpreters {
            Interpreters::Ready(list) => list,
            Interpreters::Loading(_) => &[],
        }
    }

    pub fn python_loading(&self) -> bool {
        matches!(self.interpreters, Interpreters::Loading(_))
    }

    pub fn python_default(&self) -> usize {
        preselect(self.python_list(), self.python_hint.as_deref())
    }

    /// The interpreter the form currently points at, if discovery is done.
    pub fn chosen_python(&self, form: &CreateForm) -> Option<&Interpreter> {
        let list = self.python_list();
        let index = form.python_choice.unwrap_or_else(|| self.python_default());
        list.get(index)
    }

    pub fn poll_interpreters(&mut self) {
        let Interpreters::Loading(receiver) = &self.interpreters else {
            return;
        };
        match receiver.try_recv() {
            Ok(list) => self.interpreters = Interpreters::Ready(list),
            Err(TryRecvError::Disconnected) => self.interpreters = Interpreters::Ready(Vec::new()),
            Err(TryRecvError::Empty) => {}
        }
    }

    pub fn selected_venv(&self) -> Option<&Venv> {
        self.venvs.get(self.selected)
    }

    /// True when the run command starts with a `.py` path that does not exist.
    pub fn run_target_missing(&self) -> bool {
        self.run_line
            .split_whitespace()
            .next()
            .is_some_and(|target| target.ends_with(".py") && !self.project.join(target).exists())
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        self.status = None;
        match mem::replace(&mut self.screen, Screen::List) {
            Screen::List => self.handle_list_key(key),
            Screen::Create(mut form) => {
                let count = self.python_list().len();
                match form.handle_key(key, count, self.python_default()) {
                    FormEvent::None => self.screen = Screen::Create(form),
                    FormEvent::Cancel => {}
                    FormEvent::Submit => self.submit_create(form),
                }
            }
            Screen::EditRun(mut input) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter if input.value.trim().is_empty() => {
                    self.status = Some("The run command must not be empty.".to_owned());
                    self.screen = Screen::EditRun(input);
                }
                KeyCode::Enter => self.run_line = input.value.trim().to_owned(),
                _ => {
                    input.handle_key(key);
                    self.screen = Screen::EditRun(input);
                }
            },
            // Only an explicit `y` deletes; Enter must not, so a stray Enter
            // after pressing `d` cannot destroy anything.
            Screen::ConfirmDelete(confirm) => match key.code {
                KeyCode::Char('y' | 'Y') => self.delete_confirmed(&confirm),
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    self.status = Some(format!("Kept {}.", confirm.name));
                }
                _ => self.screen = Screen::ConfirmDelete(confirm),
            },
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) {
        let rows = self.venvs.len() + 1;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = step_index(self.selected, rows, false)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = step_index(self.selected, rows, true)
            }
            KeyCode::Enter if self.selected_venv().is_none() => self.open_create(),
            KeyCode::Enter | KeyCode::Char('s') => self.start_selected(),
            KeyCode::Char('n') => self.open_create(),
            KeyCode::Char('d') | KeyCode::Delete => self.open_delete(),
            KeyCode::Char('e') => self.screen = Screen::EditRun(TextInput::new(&self.run_line)),
            KeyCode::Char('r') => {
                self.rescan(None);
                self.status = Some("Rescanned the project.".to_owned());
            }
            _ => {}
        }
    }

    fn open_create(&mut self) {
        self.screen = Screen::Create(CreateForm::new(&self.project));
    }

    fn open_delete(&mut self) {
        let Some(venv) = self.selected_venv() else {
            self.status = Some("Select a venv to delete first.".to_owned());
            return;
        };
        let symlink_target = symlink_target(&venv.path);
        self.screen = Screen::ConfirmDelete(DeleteConfirm {
            name: venv.name.clone(),
            path: venv.path.clone(),
            python_version: venv.python_version.clone(),
            healthy: venv.healthy,
            stats: symlink_target.is_none().then(|| dir_stats(&venv.path)),
            symlink_target,
        });
    }

    fn delete_confirmed(&mut self, confirm: &DeleteConfirm) {
        let result = delete_venv(&self.project, &confirm.name);
        // Rescan either way: after a failure or a vanished folder the list
        // should show what is really there.
        self.rescan(None);
        self.selected = if self.venvs.is_empty() {
            0
        } else {
            self.selected.min(self.venvs.len() - 1)
        };
        self.status = Some(match result {
            Ok(()) if self.venvs.is_empty() => {
                format!("Deleted {}. Press n to create a new venv.", confirm.name)
            }
            Ok(()) => format!("Deleted {}.", confirm.name),
            Err(message) => message,
        });
    }

    fn start_selected(&mut self) {
        let Some(venv) = self.selected_venv() else {
            return;
        };
        if !venv.healthy {
            self.status = Some(format!(
                "{} is broken (its Python executable is missing). Create a new venv instead.",
                venv.name
            ));
            return;
        }
        match plan_start(&self.project, &venv.path, &self.run_line) {
            Ok(step) => self.pending = Some(Action::Start { step }),
            Err(message) => self.status = Some(message),
        }
    }

    fn submit_create(&mut self, mut form: CreateForm) {
        match self.build_create(&form) {
            Ok(action) => self.pending = Some(action),
            Err(message) => {
                form.error = Some(message);
                self.screen = Screen::Create(form);
            }
        }
    }

    fn build_create(&self, form: &CreateForm) -> Result<Action, String> {
        let name = form.name.value.trim();
        validate_venv_name(&self.project, name)?;
        if self.python_loading() {
            return Err(
                "Still looking for Python interpreters — try again in a moment.".to_owned(),
            );
        }
        let Some(interpreter) = self.chosen_python(form) else {
            return Err(
                "No usable Python found. Install Python (with its venv module) first.".to_owned(),
            );
        };
        let steps = plan_create(
            &self.project,
            &CreateRequest {
                name,
                interpreter: &interpreter.path,
                dependencies: form.dependencies(),
            },
        );
        Ok(Action::Create {
            name: name.to_owned(),
            steps,
        })
    }

    /// Called after the terminal hand-over: updates status and venv list.
    pub fn finish(&mut self, action: &Action, outcome: &RunOutcome, quit: bool) {
        self.quit |= quit;
        match action {
            Action::Create { name, .. } => {
                self.status = Some(match outcome.failed_step {
                    None => format!("Created {name}. Press Enter to start."),
                    Some(0) => {
                        // We created this directory just now (the name was
                        // validated as free), so drop the half-built venv.
                        let _ = fs::remove_dir_all(self.project.join(name));
                        format!("Creating {name} failed ({}); nothing was left behind.", outcome.summary)
                    }
                    Some(_) => format!(
                        "Created {name}, but installing dependencies failed ({}). The venv was kept.",
                        outcome.summary
                    ),
                });
                self.rescan(Some(name));
            }
            Action::Start { .. } => {
                self.status = Some(format!("Last run {}.", outcome.summary));
                self.rescan(None);
            }
        }
    }

    fn rescan(&mut self, select: Option<&str>) {
        let previous = self.selected_venv().map(|venv| venv.name.clone());
        self.venvs = scan_venvs(&self.project);
        self.markers = project_markers(&self.project);
        let wanted = select.map(str::to_owned).or(previous);
        self.selected = wanted
            .and_then(|name| self.venvs.iter().position(|venv| venv.name == name))
            .unwrap_or_else(|| self.selected.min(self.venvs.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use tempfile::{tempdir, TempDir};
    use venv_runner::project::{venv_bin_dir, venv_python};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn press(app: &mut App, codes: &[KeyCode]) {
        for code in codes {
            app.handle_key(key(*code));
        }
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
    }

    fn interpreter(version: &str, is_default: bool) -> Interpreter {
        Interpreter {
            version: version.to_owned(),
            path: PathBuf::from(format!("/py/{version}/bin/python3")),
            source: "PATH",
            is_default,
        }
    }

    fn make_venv(project: &Path, name: &str, with_python: bool) {
        let dir = project.join(name);
        fs::create_dir_all(venv_bin_dir(&dir)).unwrap();
        fs::write(dir.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        if with_python {
            fs::write(venv_python(&dir), "").unwrap();
        }
    }

    fn project_with_main() -> TempDir {
        let project = tempdir().unwrap();
        fs::write(project.path().join("main.py"), "print('hi')\n").unwrap();
        project
    }

    fn new_app(project: &TempDir) -> App {
        App::new(
            project.path().to_path_buf(),
            Interpreters::Ready(vec![
                interpreter("3.14.7", false),
                interpreter("3.12.15", true),
            ]),
        )
    }

    #[test]
    fn a_project_without_a_venv_opens_the_create_form_with_a_free_default_name() {
        let project = project_with_main();

        let app = new_app(&project);

        let Screen::Create(form) = &app.screen else {
            panic!("expected the create form");
        };
        assert_eq!(form.name.value, ".venv");
    }

    #[test]
    fn existing_venvs_are_listed_and_the_first_healthy_one_is_selected() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", false);
        make_venv(project.path(), ".venv-abc", true);

        let app = new_app(&project);

        assert!(matches!(app.screen, Screen::List));
        assert_eq!(app.venvs.len(), 2);
        assert_eq!(app.selected_venv().unwrap().name, ".venv-abc");
    }

    #[test]
    fn start_runs_main_py_with_the_selected_venv() {
        let project = project_with_main();
        make_venv(project.path(), ".venv-abc", true);
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Enter]);

        let Some(Action::Start { step }) = &app.pending else {
            panic!("expected a start action");
        };
        assert_eq!(step.display(), ".venv-abc/bin/python main.py");
        assert_eq!(step.cwd, project.path());
    }

    #[test]
    fn starting_a_broken_venv_is_refused_with_an_explanation() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", false);
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Char('s')]);

        assert!(app.pending.is_none());
        assert!(app.status.as_deref().unwrap().contains("broken"));
    }

    #[test]
    fn the_run_command_can_be_edited_and_empty_input_is_rejected() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Char('e')]);
        assert!(matches!(app.screen, Screen::EditRun(_)));
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        press(&mut app, &[KeyCode::Enter]);
        assert!(app.status.as_deref().unwrap().contains("empty"));
        assert!(matches!(app.screen, Screen::EditRun(_)));

        type_text(&mut app, "-m uvicorn app:app");
        press(&mut app, &[KeyCode::Enter]);
        assert_eq!(app.run_line, "-m uvicorn app:app");
        press(&mut app, &[KeyCode::Char('s')]);

        let Some(Action::Start { step }) = &app.pending else {
            panic!("expected a start action");
        };
        assert_eq!(step.args, ["-m", "uvicorn", "app:app"]);
    }

    #[test]
    fn a_missing_run_target_is_flagged() {
        let project = tempdir().unwrap();
        make_venv(project.path(), ".venv", true);

        let mut app = new_app(&project);
        assert!(app.run_target_missing(), "no main.py in this project");

        fs::write(project.path().join("main.py"), "").unwrap();
        app.run_line = "main.py --flag".to_owned();
        assert!(!app.run_target_missing());
        app.run_line = "-m pytest".to_owned();
        assert!(!app.run_target_missing());
    }

    #[test]
    fn creating_a_venv_uses_the_typed_name_the_chosen_python_and_dependencies() {
        let project = project_with_main();
        fs::write(project.path().join("requirements.txt"), "").unwrap();
        let mut app = new_app(&project);

        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        type_text(&mut app, ".venv-abc");
        press(&mut app, &[KeyCode::Tab, KeyCode::Right]);
        press(&mut app, &[KeyCode::Enter]);

        let Some(Action::Create { name, steps }) = &app.pending else {
            panic!("expected a create action");
        };
        assert_eq!(name, ".venv-abc");
        // 3.12.15 is the default (index 1); Right wraps to index 0 = 3.14.7.
        assert_eq!(
            steps[0].display(),
            "/py/3.14.7/bin/python3 -m venv .venv-abc"
        );
        assert_eq!(
            steps[1].display(),
            ".venv-abc/bin/python -m pip install -r requirements.txt"
        );
    }

    #[test]
    fn the_dependency_choice_can_be_switched_to_none() {
        let project = project_with_main();
        fs::write(project.path().join("requirements.txt"), "").unwrap();
        let mut app = new_app(&project);

        press(
            &mut app,
            &[KeyCode::Tab, KeyCode::Tab, KeyCode::Right, KeyCode::Enter],
        );

        let Some(Action::Create { steps, .. }) = &app.pending else {
            panic!("expected a create action");
        };
        assert_eq!(steps.len(), 1, "no install step when 'none' is chosen");
    }

    #[test]
    fn python_version_hint_decides_the_preselected_interpreter() {
        let project = project_with_main();
        fs::write(project.path().join(".python-version"), "3.14\n").unwrap();
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Enter]);

        let Some(Action::Create { steps, .. }) = &app.pending else {
            panic!("expected a create action");
        };
        assert!(steps[0].display().starts_with("/py/3.14.7/bin/python3"));
    }

    #[test]
    fn invalid_or_taken_names_keep_the_form_open_with_an_error() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);
        press(&mut app, &[KeyCode::Char('n')]);
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        type_text(&mut app, "bad name");

        press(&mut app, &[KeyCode::Enter]);

        assert!(app.pending.is_none());
        let Screen::Create(form) = &app.screen else {
            panic!("form should stay open");
        };
        assert!(form.error.is_some());

        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        type_text(&mut app, ".venv");
        press(&mut app, &[KeyCode::Enter]);
        let Screen::Create(form) = &app.screen else {
            panic!("form should stay open");
        };
        assert!(form.error.as_deref().unwrap().contains("already exists"));
    }

    #[test]
    fn creating_while_interpreters_are_still_loading_waits_and_works_once_ready() {
        let project = project_with_main();
        let (sender, receiver) = mpsc::channel();
        let mut app = App::new(
            project.path().to_path_buf(),
            Interpreters::Loading(receiver),
        );

        press(&mut app, &[KeyCode::Enter]);
        let Screen::Create(form) = &app.screen else {
            panic!("form should stay open");
        };
        assert!(form.error.as_deref().unwrap().contains("Still looking"));

        sender.send(vec![interpreter("3.12.15", true)]).unwrap();
        app.poll_interpreters();
        press(&mut app, &[KeyCode::Enter]);

        assert!(matches!(app.pending, Some(Action::Create { .. })));
    }

    #[test]
    fn no_interpreters_means_a_clear_error_instead_of_a_broken_command() {
        let project = project_with_main();
        let mut app = App::new(
            project.path().to_path_buf(),
            Interpreters::Ready(Vec::new()),
        );

        press(&mut app, &[KeyCode::Enter]);

        assert!(app.pending.is_none());
        let Screen::Create(form) = &app.screen else {
            panic!("form should stay open");
        };
        assert!(form.error.as_deref().unwrap().contains("No usable Python"));
    }

    #[test]
    fn escape_leaves_the_form_and_quit_keys_work() {
        let project = project_with_main();
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Esc]);
        assert!(matches!(app.screen, Screen::List));
        assert!(!app.quit);

        press(&mut app, &[KeyCode::Char('q')]);
        assert!(app.quit);

        let mut app = new_app(&project);
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
    }

    #[test]
    fn finishing_a_failed_creation_removes_the_half_built_directory() {
        let project = project_with_main();
        let mut app = new_app(&project);
        fs::create_dir(project.path().join(".venv")).unwrap();
        let action = Action::Create {
            name: ".venv".to_owned(),
            steps: Vec::new(),
        };
        let outcome = RunOutcome {
            failed_step: Some(0),
            summary: "exited with code 1".to_owned(),
        };

        app.finish(&action, &outcome, false);

        assert!(!project.path().join(".venv").exists());
        assert!(app.status.as_deref().unwrap().contains("failed"));
    }

    #[test]
    fn a_failed_dependency_install_keeps_the_new_venv_and_selects_it() {
        let project = project_with_main();
        make_venv(project.path(), ".venv-a", true);
        let mut app = new_app(&project);
        make_venv(project.path(), ".venv-b", true);
        let action = Action::Create {
            name: ".venv-b".to_owned(),
            steps: Vec::new(),
        };
        let outcome = RunOutcome {
            failed_step: Some(1),
            summary: "exited with code 1".to_owned(),
        };

        app.finish(&action, &outcome, false);

        assert!(project.path().join(".venv-b").exists());
        assert_eq!(app.selected_venv().unwrap().name, ".venv-b");
        assert!(app.status.as_deref().unwrap().contains("kept"));
    }

    #[test]
    fn text_input_edits_at_the_cursor_with_multibyte_characters() {
        let mut input = TextInput::new("aéb");
        input.handle_key(key(KeyCode::Left));
        input.handle_key(key(KeyCode::Char('ü')));
        assert_eq!(input.value, "aéüb");
        input.handle_key(key(KeyCode::Backspace));
        input.handle_key(key(KeyCode::Backspace));
        assert_eq!(input.value, "ab");
        input.handle_key(key(KeyCode::Home));
        input.handle_key(key(KeyCode::Delete));
        assert_eq!(input.value, "b");
        assert_eq!(input.display(), "▏b");
    }

    fn confirm(app: &App) -> &DeleteConfirm {
        let Screen::ConfirmDelete(confirm) = &app.screen else {
            panic!("expected the delete confirmation");
        };
        confirm
    }

    #[test]
    fn d_opens_a_confirmation_that_says_what_would_be_removed_and_removes_nothing() {
        let project = project_with_main();
        make_venv(project.path(), ".venv-abc", true);
        fs::write(project.path().join(".venv-abc/lib.txt"), "12345").unwrap();
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Char('d')]);

        let dialog = confirm(&app);
        assert_eq!(dialog.name, ".venv-abc");
        assert_eq!(dialog.path, project.path().join(".venv-abc"));
        assert_eq!(dialog.python_version.as_deref(), Some("3.12.4"));
        assert_eq!(
            dialog.stats,
            Some(DirStats {
                files: 3,
                bytes: 22
            })
        );
        assert_eq!(dialog.symlink_target, None);
        assert!(project.path().join(".venv-abc").exists());
    }

    #[test]
    fn only_y_deletes_and_every_other_key_leaves_the_venv_alone() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);

        press(
            &mut app,
            &[
                KeyCode::Char('d'),
                KeyCode::Enter,
                KeyCode::Char('x'),
                KeyCode::Char(' '),
            ],
        );
        assert!(
            matches!(app.screen, Screen::ConfirmDelete(_)),
            "dialog stays open"
        );
        assert!(
            project.path().join(".venv").exists(),
            "Enter must not confirm"
        );

        press(&mut app, &[KeyCode::Char('n')]);
        assert!(matches!(app.screen, Screen::List));
        assert!(project.path().join(".venv").exists());
        assert!(app.status.as_deref().unwrap().contains("Kept .venv"));

        press(&mut app, &[KeyCode::Char('d'), KeyCode::Esc]);
        assert!(matches!(app.screen, Screen::List));
        assert!(project.path().join(".venv").exists());

        press(&mut app, &[KeyCode::Char('d'), KeyCode::Char('y')]);
        assert!(!project.path().join(".venv").exists());
        assert!(
            project.path().join("main.py").exists(),
            "project files are untouched"
        );
        assert!(matches!(app.screen, Screen::List));
        assert!(app.venvs.is_empty());
    }

    #[test]
    fn deleting_the_last_venv_hints_at_creating_a_new_one() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Delete, KeyCode::Char('y')]);

        assert!(app
            .status
            .as_deref()
            .unwrap()
            .contains("Deleted .venv. Press n"));
        assert_eq!(app.selected, 0, "the only row left is the new-venv row");
        assert!(app.selected_venv().is_none());
    }

    #[test]
    fn deleting_the_last_row_selects_its_neighbour() {
        let project = project_with_main();
        for name in [".venv", ".venv-a", ".venv-b"] {
            make_venv(project.path(), name, true);
        }
        let mut app = new_app(&project);
        press(&mut app, &[KeyCode::Down, KeyCode::Down]);
        assert_eq!(app.selected_venv().unwrap().name, ".venv-b");

        press(&mut app, &[KeyCode::Char('d'), KeyCode::Char('y')]);

        assert_eq!(app.venvs.len(), 2);
        assert_eq!(app.selected_venv().unwrap().name, ".venv-a");
        assert_eq!(app.status.as_deref(), Some("Deleted .venv-b."));
    }

    #[test]
    fn d_on_the_new_venv_row_only_explains() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);
        press(&mut app, &[KeyCode::Down]);
        assert!(app.selected_venv().is_none());

        press(&mut app, &[KeyCode::Char('d')]);

        assert!(matches!(app.screen, Screen::List));
        assert!(app.status.as_deref().unwrap().contains("Select a venv"));
        assert!(project.path().join(".venv").exists());
    }

    #[test]
    fn a_broken_venv_can_be_deleted_through_the_dialog() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", false);
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Char('d')]);
        assert!(!confirm(&app).healthy);
        press(&mut app, &[KeyCode::Char('y')]);

        assert!(!project.path().join(".venv").exists());
    }

    #[test]
    fn a_venv_that_vanished_meanwhile_is_reported_and_the_list_refreshed() {
        let project = project_with_main();
        make_venv(project.path(), ".venv", true);
        let mut app = new_app(&project);
        press(&mut app, &[KeyCode::Char('d')]);
        fs::remove_dir_all(project.path().join(".venv")).unwrap();

        press(&mut app, &[KeyCode::Char('y')]);

        assert!(app
            .status
            .as_deref()
            .unwrap()
            .contains("no longer a virtual environment"));
        assert!(app.venvs.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_venv_is_labelled_as_such_and_only_the_link_is_removed() {
        let project = project_with_main();
        let elsewhere = tempdir().unwrap();
        make_venv(elsewhere.path(), "real", true);
        let target = elsewhere.path().join("real");
        std::os::unix::fs::symlink(&target, project.path().join(".venv")).unwrap();
        let mut app = new_app(&project);

        press(&mut app, &[KeyCode::Char('d')]);
        let dialog = confirm(&app);
        assert_eq!(dialog.symlink_target.as_deref(), Some(target.as_path()));
        assert_eq!(
            dialog.stats, None,
            "the size of the target is not what gets deleted"
        );
        press(&mut app, &[KeyCode::Char('y')]);

        assert!(app.venvs.is_empty());
        assert!(
            target.join("pyvenv.cfg").is_file(),
            "the linked venv must survive"
        );
    }
}
