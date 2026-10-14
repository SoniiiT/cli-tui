//! Rendering. Pure functions of [`App`]; no state changes here.

use std::env;
use std::path::Path;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use venv_runner::project::venv_python;

use crate::app::{
    App, CreateForm, DeleteConfirm, Field, Screen, TextInput, UpdateField, UpdateForm,
};

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    // The key list grows with every feature, so let it wrap on narrow
    // terminals instead of cutting off the last keys.
    let hints = key_hints(app);
    let hint_rows = wrapped_rows(
        Line::from(hints).width(),
        usize::from(frame.area().width.saturating_sub(2)).max(1),
    );
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(u16::try_from(hint_rows + 2).unwrap_or(3)),
            Constraint::Length(3),
        ])
        .split(frame.area());
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(layout[0]);
    draw_venv_list(frame, app, main[0]);
    draw_details(frame, app, main[1]);

    frame.render_widget(
        Paragraph::new(hints)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Key bindings "),
            )
            .wrap(Wrap { trim: true }),
        layout[1],
    );
    let (status, style) = match &app.status {
        Some(message) => (message.as_str(), Style::default().fg(Color::Yellow)),
        None => ("Ready", Style::default().fg(Color::DarkGray)),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(status, style))
            .block(Block::default().borders(Borders::ALL).title(" Status "))
            .wrap(Wrap { trim: true }),
        layout[2],
    );

    match &app.screen {
        Screen::List => {}
        Screen::Create(form) => draw_create(frame, app, form),
        Screen::EditRun(input) => draw_edit_run(frame, app, input),
        Screen::ConfirmDelete(confirm) => draw_confirm_delete(frame, confirm),
        Screen::Update(form) => draw_update(frame, app, form),
    }
}

fn draw_venv_list(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let mut items: Vec<ListItem<'_>> = app
        .venvs
        .iter()
        .map(|venv| {
            let (mark, color) = if venv.healthy {
                ("✔", Color::Green)
            } else {
                ("✘ broken", Color::Red)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<16}", venv.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<9}", venv.python_version.as_deref().unwrap_or("?")),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(mark, Style::default().fg(color)),
            ]))
        })
        .collect();
    items.push(ListItem::new(Line::from(Span::styled(
        "+ New virtual environment…",
        Style::default().fg(Color::Green),
    ))));
    let title = format!(" Virtual environments · {} ", app.venvs.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_details(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let label = |text: &'static str| {
        Span::styled(
            text,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
    };
    let markers = if app.markers.is_empty() {
        "no Python project files found (still usable)".to_owned()
    } else {
        app.markers.join(" · ")
    };
    let mut lines = vec![
        Line::from(vec![
            label("Project   "),
            Span::raw(fit_left(
                &tilde(&app.project),
                usize::from(area.width.saturating_sub(2 + 10)),
            )),
        ]),
        Line::from(vec![label("Detected  "), Span::raw(markers)]),
        Line::from(""),
    ];
    match app.selected_venv() {
        Some(venv) => {
            lines.push(Line::from(vec![
                label("Venv      "),
                Span::raw(format!(
                    "{} (Python {})",
                    venv.name,
                    venv.python_version.as_deref().unwrap_or("?")
                )),
            ]));
            if venv.healthy {
                let python = venv_python(&venv.path);
                let python = python.strip_prefix(&app.project).unwrap_or(&python);
                lines.push(Line::from(vec![
                    label("Will run  "),
                    Span::styled(
                        format!("$ {} {}", python.display(), app.run_line),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(
                    "Enter starts it in this terminal; you return here afterwards.",
                ));
                if app.run_target_missing() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "The script in the run command does not exist here — press e to change it.",
                        Style::default().fg(Color::Yellow),
                    )));
                }
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "This venv is broken: its Python executable is gone (the base Python it \
                     was created from was probably removed). Press d to delete it, then n \
                     to create a new one.",
                    Style::default().fg(Color::Red),
                )));
            }
        }
        None => {
            lines.push(Line::from(
                "Create a new virtual environment in this project.",
            ));
            lines.push(Line::from(
                "Press Enter (or n) to choose name, Python and dependencies.",
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Run "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn key_hints(app: &App) -> &'static str {
    match &app.screen {
        Screen::List if app.selected_venv().is_some() => {
            "↑/↓ select · Enter start · n new · u update · d delete · e edit run command · r rescan · q quit"
        }
        Screen::List => "↑/↓ select · Enter new venv · e edit run command · r rescan · q quit",
        Screen::Create(_) => "Tab/↑/↓ field · ←/→ change choice · Enter create · Esc cancel",
        Screen::EditRun(_) => "Enter save · Esc cancel · Ctrl+U clear",
        Screen::ConfirmDelete(_) => "y delete · n or Esc keep it",
        Screen::Update(_) => {
            "Tab/↑/↓ field · ←/→ or Space change · Enter update · Esc cancel"
        }
    }
}

fn draw_update(frame: &mut Frame<'_>, app: &App, form: &UpdateForm) {
    let marker = |field: UpdateField| if form.field == field { "› " } else { "  " };
    let choice_style = |field: UpdateField| {
        if form.field == field {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    };
    let dim = Style::default().fg(Color::DarkGray);
    let dependencies = form
        .dependencies()
        .map_or_else(|| "none".to_owned(), |source| source.label());
    let packages = if form.dependencies().is_none() {
        "unused without a dependency source".to_owned()
    } else if form.upgrade_packages {
        "upgrade to the newest versions allowed".to_owned()
    } else {
        "install what is missing only".to_owned()
    };
    let version = form.python_version.as_deref().unwrap_or("?");

    let mut lines = vec![
        Line::from(Span::styled(
            format!("Update {} (Python {version})", form.name),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw(format!(
                "{}Dependencies: ",
                marker(UpdateField::Dependencies)
            )),
            Span::styled(
                format!("◀ {dependencies} ▶"),
                choice_style(UpdateField::Dependencies),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!("{}Upgrade pip:  ", marker(UpdateField::UpgradePip))),
            Span::styled(
                format!("◀ {} ▶", if form.upgrade_pip { "yes" } else { "no" }),
                choice_style(UpdateField::UpgradePip),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!("{}Packages:     ", marker(UpdateField::Packages))),
            Span::styled(
                format!("◀ {packages} ▶"),
                choice_style(UpdateField::Packages),
            ),
        ]),
        Line::from(""),
    ];
    let steps = form.steps(&app.project);
    if steps.is_empty() {
        lines.push(Line::from(Span::styled(
            "Nothing is selected, so nothing would run.",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        lines.push(Line::from(Span::styled("Will run:", dim)));
        lines.extend(
            steps
                .iter()
                .map(|step| Line::from(Span::raw(format!("  $ {}", step.display())))),
        );
    }
    if form.dependency_options.len() == 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No requirements file or installable pyproject.toml found: only pip itself can be updated.",
            dim,
        )));
    }
    if let Some(target) = &form.symlink_target {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "This venv is a symlink to {}: packages are installed there.",
                tilde(target)
            ),
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Packages that are no longer required stay installed; for a clean slate delete the venv and create a new one.",
        dim,
    )));
    lines.push(Line::from(Span::styled(
        form.error.as_deref().unwrap_or(""),
        Style::default().fg(Color::Red),
    )));

    let area = modal_rect(80, &lines, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Update virtual environment "),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_confirm_delete(frame: &mut Frame<'_>, confirm: &DeleteConfirm) {
    let label = |text: &'static str| {
        Span::styled(
            text,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
    };
    let version = confirm.python_version.as_deref().unwrap_or("?");
    let state = if confirm.healthy { "" } else { " · broken" };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Delete {}?", confirm.name),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![label("Folder  "), Span::raw(tilde(&confirm.path))]),
        Line::from(vec![
            label("Python  "),
            Span::raw(format!("{version}{state}")),
        ]),
    ];
    match (&confirm.symlink_target, confirm.stats) {
        (Some(target), _) => {
            lines.push(Line::from(vec![
                label("Link to "),
                Span::raw(tilde(target)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(
                "This folder is a symlink: only the link is removed, the venv it points to stays.",
            ));
        }
        (None, Some(stats)) => {
            lines.push(Line::from(vec![
                label("Size    "),
                Span::raw(format!(
                    "{} files · {}",
                    stats.files,
                    format_size(stats.bytes)
                )),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(
                "This removes the folder permanently. Your project files are not touched.",
            ));
        }
        (None, None) => {}
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "y",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" delete · "),
        Span::styled("n", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" or "),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" keep it"),
    ]));
    let area = modal_rect(80, &lines, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" Delete virtual environment "),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn draw_create(frame: &mut Frame<'_>, app: &App, form: &CreateForm) {
    let marker = |field: Field| if form.field == field { "› " } else { "  " };
    let choice_style = |field: Field| {
        if form.field == field {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    };

    let (python_line, python_path) = if app.python_loading() {
        ("looking for Python interpreters…".to_owned(), String::new())
    } else if let Some(python) = app.chosen_python(form) {
        let default = if python.is_default { " (default)" } else { "" };
        (
            format!("◀ {}{default} ▶", python.label()),
            tilde(&python.path),
        )
    } else {
        ("none found".to_owned(), String::new())
    };
    let dependencies = form
        .dependencies()
        .map_or_else(|| "none (empty venv)".to_owned(), |source| source.label());

    let mut lines = vec![
        Line::from(format!(
            "{}Name:         {}",
            marker(Field::Name),
            form.name.display()
        )),
        Line::from(vec![
            Span::raw(format!("{}Python:       ", marker(Field::Python))),
            Span::styled(python_line, choice_style(Field::Python)),
        ]),
        Line::from(Span::styled(
            format!("                {python_path}"),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::raw(format!("{}Dependencies: ", marker(Field::Dependencies))),
            Span::styled(
                format!("◀ {dependencies} ▶"),
                choice_style(Field::Dependencies),
            ),
        ]),
        Line::from(""),
    ];
    let hints: Vec<String> = [
        app.python_hint
            .as_ref()
            .map(|version| format!(".python-version asks for {version}")),
        app.requires_python
            .as_ref()
            .map(|spec| format!("pyproject requires-python {spec}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !hints.is_empty() {
        lines.push(Line::from(Span::styled(
            hints.join(" · "),
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(Span::styled(
        form.error.as_deref().unwrap_or(""),
        Style::default().fg(Color::Red),
    )));

    let area = modal_rect(80, &lines, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(format!(
                " New virtual environment · {} ",
                project_name(&app.project)
            )))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_edit_run(frame: &mut Frame<'_>, app: &App, input: &TextInput) {
    let python = app
        .selected_venv()
        .map(|venv| venv_python(&venv.path))
        .map(|path| {
            path.strip_prefix(&app.project)
                .map_or_else(|_| path.display().to_string(), |p| p.display().to_string())
        })
        .unwrap_or_else(|| "python".to_owned());
    let lines = vec![
        Line::from("Arguments for the venv's Python, e.g.:"),
        Line::from(Span::styled(
            "  main.py   ·   main.py --debug   ·   -m uvicorn app:app --reload",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(format!("$ {python} {}", input.display())),
        Line::from(""),
        Line::from(Span::styled(
            app.status.as_deref().unwrap_or(""),
            Style::default().fg(Color::Red),
        )),
    ];
    let area = modal_rect(80, &lines, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Run command "),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// `/Users/me/project` → `~/project`.
fn tilde(path: &Path) -> String {
    let home = env::var_os("HOME").map(std::path::PathBuf::from);
    match home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// A box `width_percent` wide and `height_rows` tall, centered in `area`.
fn centered_rect(width_percent: u16, height_rows: u16, area: Rect) -> Rect {
    let height = height_rows.min(area.height);
    let width = u16::try_from(u32::from(area.width) * u32::from(width_percent) / 100)
        .unwrap_or(area.width)
        .max(1);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// A modal `width_percent` wide and tall enough for `lines` *including the
/// rows that wrapping adds*, so nothing is ever cut off. That matters most for
/// the delete dialog, whose last line is the key that confirms. Only the
/// screen height can still limit it.
fn modal_rect(width_percent: u16, lines: &[Line<'_>], area: Rect) -> Rect {
    let inner = usize::from(
        centered_rect(width_percent, 1, area)
            .width
            .saturating_sub(2),
    )
    .max(1);
    let rows: usize = lines
        .iter()
        .map(|line| wrapped_rows(line.width(), inner))
        .sum();
    centered_rect(
        width_percent,
        u16::try_from(rows + 2).unwrap_or(u16::MAX),
        area,
    )
}

/// Word wrapping can need more rows than `ceil(width / inner)`, so a line that
/// wraps gets one extra row of slack. Overestimating only adds blank space.
fn wrapped_rows(width: usize, inner: usize) -> usize {
    if width <= inner {
        1
    } else {
        width.div_ceil(inner) + 1
    }
}

/// Keeps the end of a long path (the interesting part) and elides the start.
fn fit_left(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max || max < 2 {
        return text.to_owned();
    }
    let tail: String = text.chars().skip(count - (max - 1)).collect();
    format!("…{tail}")
}

fn project_name(project: &Path) -> String {
    project.file_name().map_or_else(
        || project.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Interpreters;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::fs;
    use tempfile::tempdir;
    use venv_runner::project::venv_bin_dir;
    use venv_runner::python::Interpreter;

    fn render(app: &App) -> String {
        render_sized(app, 110, 32)
    }

    fn render_sized(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        buffer.content().iter().map(|cell| cell.symbol()).collect()
    }

    fn interpreters() -> Interpreters {
        Interpreters::Ready(vec![Interpreter {
            version: "3.12.15".to_owned(),
            path: "/py/bin/python3".into(),
            source: "uv",
            is_default: true,
        }])
    }

    #[test]
    fn the_list_shows_venvs_the_command_that_will_run_and_the_new_venv_row() {
        let project = tempdir().unwrap();
        fs::write(project.path().join("main.py"), "").unwrap();
        let venv = project.path().join(".venv-abc");
        fs::create_dir_all(venv_bin_dir(&venv)).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&venv), "").unwrap();
        let app = App::new(project.path().to_path_buf(), interpreters());

        let screen = render(&app);

        assert!(screen.contains(".venv-abc"));
        assert!(screen.contains("3.12.4"));
        assert!(screen.contains("+ New virtual environment"));
        assert!(screen.contains("$ .venv-abc/bin/python main.py"));
    }

    #[test]
    fn the_create_form_shows_name_python_and_dependencies() {
        let project = tempdir().unwrap();
        fs::write(project.path().join("requirements.txt"), "").unwrap();
        let app = App::new(project.path().to_path_buf(), interpreters());

        let screen = render(&app);

        assert!(screen.contains("New virtual environment"));
        assert!(screen.contains(".venv"));
        assert!(screen.contains("3.12.15 · uv (default)"));
        assert!(screen.contains("requirements.txt"));
    }

    #[test]
    fn a_broken_venv_is_flagged_in_red_text_instead_of_offering_a_command() {
        let project = tempdir().unwrap();
        let venv = project.path().join(".venv");
        fs::create_dir_all(&venv).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        let app = App::new(project.path().to_path_buf(), interpreters());

        let screen = render(&app);

        assert!(screen.contains("broken"));
        assert!(!screen.contains("Will run"));
    }

    fn press_d(app: &mut App) {
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ));
    }

    #[test]
    fn the_delete_dialog_names_the_venv_its_size_and_the_only_confirming_key() {
        let project = tempdir().unwrap();
        let venv = project.path().join(".venv-abc");
        fs::create_dir_all(venv_bin_dir(&venv)).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&venv), "").unwrap();
        fs::write(venv.join("big.bin"), vec![0_u8; 3 * 1024 * 1024]).unwrap();
        let mut app = App::new(project.path().to_path_buf(), interpreters());

        press_d(&mut app);
        let screen = render(&app);

        assert!(screen.contains("Delete virtual environment"));
        assert!(screen.contains("Delete .venv-abc?"));
        assert!(screen.contains("3 files · 3.0 MB"), "{screen}");
        assert!(screen.contains("Your project files are not touched"));
        assert!(screen.contains("y delete · n or Esc keep it"));
        assert!(!screen.contains("symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn the_delete_dialog_explains_that_a_symlink_only_loses_the_link() {
        let project = tempdir().unwrap();
        let elsewhere = tempdir().unwrap();
        let real = elsewhere.path().join("real");
        fs::create_dir_all(venv_bin_dir(&real)).unwrap();
        fs::write(real.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&real), "").unwrap();
        std::os::unix::fs::symlink(&real, project.path().join(".venv")).unwrap();
        let mut app = App::new(project.path().to_path_buf(), interpreters());

        press_d(&mut app);
        let screen = render(&app);

        assert!(screen.contains("only the link is removed"), "{screen}");
        assert!(screen.contains("Link to"));
        assert!(!screen.contains("files ·"), "no size for a link");
    }

    fn venv_in_a_deeply_nested_project() -> (tempfile::TempDir, std::path::PathBuf) {
        let root = tempdir().unwrap();
        let project = root
            .path()
            .join("a-rather-long-customer-directory-name")
            .join("another-nested-folder-with-a-long-name")
            .join("the-actual-python-project");
        let venv = project.join(".venv-with-a-long-name");
        fs::create_dir_all(venv_bin_dir(&venv)).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&venv), "").unwrap();
        (root, project)
    }

    /// Regression: the dialog had a fixed height, so a long path that wrapped
    /// pushed the explanation and the `y`/`n` line out of the box.
    #[test]
    fn the_delete_dialog_is_never_cut_off_by_long_paths_or_narrow_terminals() {
        let (_root, project) = venv_in_a_deeply_nested_project();
        for (width, height) in [(110, 32), (80, 26), (60, 30)] {
            let mut app = App::new(project.clone(), interpreters());
            press_d(&mut app);

            let screen = render_sized(&app, width, height);

            assert!(
                screen.contains("Delete .venv-with-a-long-name?"),
                "{width}x{height}"
            );
            assert!(screen.contains("removes the folder"), "{width}x{height}");
            assert!(
                screen.contains("y delete"),
                "confirm line cut off at {width}x{height}"
            );
            assert!(screen.contains("keep it"), "{width}x{height}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_symlink_dialog_is_never_cut_off_either() {
        let (_root, project) = venv_in_a_deeply_nested_project();
        let elsewhere = tempdir().unwrap();
        let real = elsewhere
            .path()
            .join("some-shared-environments-directory")
            .join("the-real-venv-that-the-link-points-to");
        fs::create_dir_all(venv_bin_dir(&real)).unwrap();
        fs::write(real.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&real), "").unwrap();
        fs::remove_dir_all(project.join(".venv-with-a-long-name")).unwrap();
        std::os::unix::fs::symlink(&real, project.join(".venv-link")).unwrap();

        for (width, height) in [(110, 32), (80, 28), (60, 32)] {
            let mut app = App::new(project.clone(), interpreters());
            press_d(&mut app);

            let screen = render_sized(&app, width, height);

            assert!(
                screen.contains("symlink:"),
                "explanation cut off at {width}x{height}"
            );
            assert!(
                screen.contains("y delete"),
                "confirm line cut off at {width}x{height}"
            );
        }
    }

    #[test]
    fn the_create_form_keeps_its_error_line_on_a_narrow_terminal() {
        let (_root, project) = venv_in_a_deeply_nested_project();
        fs::remove_dir_all(project.join(".venv-with-a-long-name")).unwrap();
        fs::write(project.join("requirements.txt"), "").unwrap();
        fs::write(project.join(".python-version"), "3.12\n").unwrap();
        let mut app = App::new(project, interpreters());
        // The default name is valid, so clear it first to provoke the error.
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('u'),
            crossterm::event::KeyModifiers::CONTROL,
        ));
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));

        let screen = render_sized(&app, 60, 30);

        assert!(
            screen.contains("Name must not be empty."),
            "error line cut off"
        );
    }

    #[test]
    fn modals_grow_with_wrapped_lines_but_never_beyond_the_screen() {
        let area = Rect::new(0, 0, 40, 30);
        let short = [Line::from("one"), Line::from("two")];
        let long = [Line::from("x".repeat(120)), Line::from("two")];

        let short_rect = modal_rect(80, &short, area);
        let long_rect = modal_rect(80, &long, area);

        assert_eq!(short_rect.height, 2 + 2, "two rows plus borders");
        assert!(
            long_rect.height > short_rect.height + 2,
            "wrapped rows need space"
        );
        assert_eq!(
            modal_rect(80, &long, Rect::new(0, 0, 40, 6)).height,
            6,
            "capped to the screen"
        );
    }

    fn press_key(app: &mut App, ch: char) {
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(ch),
            crossterm::event::KeyModifiers::NONE,
        ));
    }

    #[test]
    fn the_update_dialog_shows_the_choices_and_the_exact_commands() {
        let project = tempdir().unwrap();
        let venv = project.path().join(".venv-abc");
        fs::create_dir_all(venv_bin_dir(&venv)).unwrap();
        fs::write(venv.join("pyvenv.cfg"), "version = 3.12.4\n").unwrap();
        fs::write(venv_python(&venv), "").unwrap();
        fs::write(project.path().join("requirements.txt"), "").unwrap();
        let mut app = App::new(project.path().to_path_buf(), interpreters());

        press_key(&mut app, 'u');
        let screen = render(&app);

        assert!(screen.contains("Update virtual environment"));
        assert!(screen.contains("Update .venv-abc (Python 3.12.4)"));
        assert!(screen.contains("◀ requirements.txt ▶"));
        assert!(screen.contains("Upgrade pip:"));
        assert!(screen.contains("$ .venv-abc/bin/python -m pip install --upgrade pip"));
        assert!(screen.contains("--upgrade-strategy eager"));
        assert!(
            screen.contains("stay installed"),
            "leftover-packages note: {screen}"
        );
    }

    /// The update dialog lists full command lines, which wrap on narrow
    /// terminals; the note and the commands must still be fully on screen.
    #[test]
    fn the_update_dialog_is_not_cut_off_on_narrow_terminals() {
        let (_root, project) = venv_in_a_deeply_nested_project();
        fs::write(project.join("requirements.txt"), "").unwrap();
        for (width, height) in [(110, 34), (80, 32), (60, 36)] {
            let mut app = App::new(project.clone(), interpreters());
            press_key(&mut app, 'u');

            let screen = render_sized(&app, width, height);

            assert!(
                screen.contains("Update virtual environment"),
                "{width}x{height}"
            );
            assert!(
                screen.contains("clean slate"),
                "last note cut off at {width}x{height}"
            );
        }
    }

    #[test]
    fn the_key_bar_wraps_instead_of_losing_keys_on_narrow_terminals() {
        let (_root, project) = venv_in_a_deeply_nested_project();
        let app = App::new(project, interpreters());

        for width in [110, 70, 50] {
            let screen = render_sized(&app, width, 30);

            // Single words: the bar wraps at spaces, so "d delete" may be split
            // over two rows while every key is still there.
            for word in [
                "select", "start", "new", "update", "delete", "rescan", "quit",
            ] {
                assert!(screen.contains(word), "{word:?} lost at width {width}");
            }
        }
    }

    #[test]
    fn sizes_are_shown_in_human_units() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(3 * 1024 * 1024), "3.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(format_size(u64::MAX).split(' ').nth(1), Some("TB"));
    }

    #[test]
    fn long_paths_keep_their_tail() {
        assert_eq!(fit_left("~/a/b", 10), "~/a/b");
        assert_eq!(fit_left("/very/long/path/to/project", 12), "…/to/project");
        assert_eq!(fit_left("abc", 1), "abc");
    }

    #[test]
    fn modals_are_centered_and_never_larger_than_the_screen() {
        let area = Rect::new(0, 0, 100, 40);
        assert_eq!(centered_rect(80, 11, area), Rect::new(10, 14, 80, 11));
        let tiny = Rect::new(0, 0, 20, 5);
        let rect = centered_rect(80, 11, tiny);
        assert!(rect.height <= tiny.height && rect.width <= tiny.width);
    }

    #[test]
    fn tilde_abbreviates_the_home_directory() {
        let home = env::var("HOME").unwrap();
        assert_eq!(tilde(Path::new(&home)), "~");
        assert_eq!(tilde(&Path::new(&home).join("a/b")), "~/a/b");
        assert_eq!(tilde(Path::new("/elsewhere/x")), "/elsewhere/x");
    }
}
