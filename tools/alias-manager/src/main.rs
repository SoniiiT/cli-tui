use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use alias_manager::{
    apply_profile_update, default_config_dir, default_profile, default_shell, discover_aliases,
    load_alias_state, persist_alias_state, prepare_profile_update, validate_alias, Alias, AliasKey,
    DiscoveredAlias, Shell,
};

struct Options {
    shell: Shell,
    config_dir: PathBuf,
    profile: Option<PathBuf>,
    setup: bool,
}

#[derive(Clone)]
enum Change {
    Create(Alias),
    Update { before: Alias, after: Alias },
    Delete(Alias),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Field {
    Name,
    Command,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum EditorPhase {
    Editing,
    Validated,
}

struct Editor {
    original: Option<Alias>,
    name: String,
    command: String,
    name_cursor: usize,
    command_cursor: usize,
    field: Field,
    phase: EditorPhase,
    error: Option<String>,
}

impl Editor {
    fn create() -> Self {
        Self {
            original: None,
            name: String::new(),
            command: String::new(),
            name_cursor: 0,
            command_cursor: 0,
            field: Field::Name,
            phase: EditorPhase::Editing,
            error: None,
        }
    }

    fn edit(alias: Alias) -> Self {
        Self {
            name: alias.name.clone(),
            command: alias.command.clone(),
            name_cursor: alias.name.chars().count(),
            command_cursor: alias.command.chars().count(),
            original: Some(alias),
            field: Field::Name,
            phase: EditorPhase::Editing,
            error: None,
        }
    }

    fn candidate(&self, shell: Shell) -> Alias {
        Alias {
            shell,
            name: self.name.clone(),
            command: self.command.clone(),
        }
    }

    fn as_change(&self, shell: Shell) -> Change {
        let after = self.candidate(shell);
        match &self.original {
            Some(before) => Change::Update {
                before: before.clone(),
                after,
            },
            None => Change::Create(after),
        }
    }

    fn display_value(&self, field: Field) -> String {
        let (value, cursor) = match field {
            Field::Name => (&self.name, self.name_cursor),
            Field::Command => (&self.command, self.command_cursor),
        };
        if self.field != field || self.phase != EditorPhase::Editing {
            return value.clone();
        }
        let byte_index = value
            .char_indices()
            .nth(cursor)
            .map_or(value.len(), |(index, _)| index);
        format!("{}▏{}", &value[..byte_index], &value[byte_index..])
    }
}

fn insert_character(value: &mut String, cursor: &mut usize, ch: char) {
    let byte_index = value
        .char_indices()
        .nth(*cursor)
        .map_or(value.len(), |(index, _)| index);
    value.insert(byte_index, ch);
    *cursor += 1;
}

fn delete_before_cursor(value: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = value
        .char_indices()
        .nth(*cursor - 1)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let end = value
        .char_indices()
        .nth(*cursor)
        .map_or(value.len(), |(index, _)| index);
    value.replace_range(start..end, "");
    *cursor -= 1;
}

enum Screen {
    List,
    Editor(Editor),
    Preview(Change),
}

struct App {
    shell: Shell,
    config_dir: PathBuf,
    aliases: Vec<Alias>,
    discovered: Vec<DiscoveredAlias>,
    suppressed: Vec<AliasKey>,
    search: String,
    searching: bool,
    selected: usize,
    screen: Screen,
    message: String,
}

#[derive(Clone)]
struct AliasRow {
    alias: Alias,
    origin: String,
}

impl App {
    fn with_discovered(
        shell: Shell,
        config_dir: PathBuf,
        aliases: Vec<Alias>,
        discovered: Vec<DiscoveredAlias>,
        suppressed: Vec<AliasKey>,
    ) -> Self {
        Self {
            shell,
            config_dir,
            aliases,
            discovered,
            suppressed,
            search: String::new(),
            searching: false,
            selected: 0,
            screen: Screen::List,
            message: "Ready — sources are shown beside discovered aliases.".into(),
        }
    }

    fn effective_rows(&self) -> Vec<AliasRow> {
        let mut rows = Vec::new();
        for discovered in &self.discovered {
            let alias = &discovered.alias;
            let key = AliasKey::new(alias.shell, alias.name.clone());
            if alias.shell != self.shell
                || self.suppressed.contains(&key)
                || self
                    .aliases
                    .iter()
                    .any(|managed| managed.shell == alias.shell && managed.name == alias.name)
            {
                continue;
            }
            rows.push(AliasRow {
                alias: alias.clone(),
                origin: discovered
                    .source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            });
        }
        rows.extend(
            self.aliases
                .iter()
                .filter(|alias| alias.shell == self.shell)
                .cloned()
                .map(|alias| AliasRow {
                    alias,
                    origin: "managed".into(),
                }),
        );
        rows.sort_by(|left, right| left.alias.name.cmp(&right.alias.name));
        rows
    }

    fn visible_rows(&self) -> Vec<AliasRow> {
        let search = self.search.to_lowercase();
        self.effective_rows()
            .into_iter()
            .filter(|row| {
                search.is_empty()
                    || row.alias.name.to_lowercase().contains(&search)
                    || row.alias.command.to_lowercase().contains(&search)
            })
            .collect()
    }

    fn visible_aliases(&self) -> Vec<Alias> {
        self.visible_rows()
            .into_iter()
            .map(|row| row.alias)
            .collect()
    }

    fn selected_alias(&self) -> Option<Alias> {
        self.visible_aliases().get(self.selected).cloned()
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        let mut transition = None;
        let mut commit_change = None;
        let mut quit = false;
        let effective_for_validation = self.effective_rows();
        match &mut self.screen {
            Screen::List => {
                let visible = self.visible_aliases();
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    quit = true;
                } else if self.searching {
                    match key.code {
                        KeyCode::Esc => {
                            self.search.clear();
                            self.searching = false;
                            self.selected = 0;
                        }
                        KeyCode::Enter => self.searching = false,
                        KeyCode::Backspace => {
                            self.search.pop();
                            self.selected = 0;
                        }
                        KeyCode::Char(ch) => {
                            self.search.push(ch);
                            self.selected = 0;
                        }
                        KeyCode::Up => {
                            self.selected = self.selected.saturating_sub(1);
                        }
                        KeyCode::Down if self.selected + 1 < visible.len() => {
                            self.selected += 1;
                        }
                        KeyCode::Down => {}
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => quit = true,
                        KeyCode::Char('/') => {
                            self.searching = true;
                            self.message = "Type to filter aliases · Enter or Esc to finish".into();
                        }
                        KeyCode::Char('a') => {
                            transition = Some(Screen::Editor(Editor::create()));
                        }
                        KeyCode::Char('d') => {
                            if let Some(alias) = self.selected_alias() {
                                transition = Some(Screen::Preview(Change::Delete(alias)));
                            }
                        }
                        KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                        KeyCode::Down => {
                            if self.selected + 1 < visible.len() {
                                self.selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(alias) = self.selected_alias() {
                                transition = Some(Screen::Editor(Editor::edit(alias)));
                            }
                        }
                        KeyCode::Esc if !self.search.is_empty() => {
                            self.search.clear();
                            self.selected = 0;
                        }
                        KeyCode::Esc => {}
                        _ => {}
                    }
                }
            }
            Screen::Editor(editor) => match key.code {
                KeyCode::Esc => {
                    transition = Some(Screen::List);
                    self.message = "Draft discarded".into();
                }
                KeyCode::Tab => {
                    editor.field = match editor.field {
                        Field::Name => Field::Command,
                        Field::Command => Field::Name,
                    };
                    editor.phase = EditorPhase::Editing;
                    editor.error = None;
                }
                KeyCode::Backspace => {
                    match editor.field {
                        Field::Name => {
                            delete_before_cursor(&mut editor.name, &mut editor.name_cursor);
                        }
                        Field::Command => {
                            delete_before_cursor(&mut editor.command, &mut editor.command_cursor);
                        }
                    }
                    editor.phase = EditorPhase::Editing;
                    editor.error = None;
                }
                KeyCode::Left => {
                    match editor.field {
                        Field::Name => editor.name_cursor = editor.name_cursor.saturating_sub(1),
                        Field::Command => {
                            editor.command_cursor = editor.command_cursor.saturating_sub(1)
                        }
                    }
                    editor.phase = EditorPhase::Editing;
                }
                KeyCode::Right => {
                    match editor.field {
                        Field::Name => {
                            editor.name_cursor =
                                (editor.name_cursor + 1).min(editor.name.chars().count())
                        }
                        Field::Command => {
                            editor.command_cursor =
                                (editor.command_cursor + 1).min(editor.command.chars().count())
                        }
                    }
                    editor.phase = EditorPhase::Editing;
                }
                KeyCode::Home => match editor.field {
                    Field::Name => editor.name_cursor = 0,
                    Field::Command => editor.command_cursor = 0,
                },
                KeyCode::End => match editor.field {
                    Field::Name => editor.name_cursor = editor.name.chars().count(),
                    Field::Command => editor.command_cursor = editor.command.chars().count(),
                },
                KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match editor.field {
                        Field::Name => {
                            insert_character(&mut editor.name, &mut editor.name_cursor, ch)
                        }
                        Field::Command => {
                            insert_character(&mut editor.command, &mut editor.command_cursor, ch)
                        }
                    }
                    editor.phase = EditorPhase::Editing;
                    editor.error = None;
                }
                KeyCode::Enter if editor.phase == EditorPhase::Editing => {
                    let candidate = editor.candidate(self.shell);
                    let validation = validate_alias(&candidate).and_then(|()| {
                        if effective_for_validation.iter().any(|row| {
                            let existing = &row.alias;
                            existing.shell == candidate.shell
                                && existing.name == candidate.name
                                && editor.original.as_ref() != Some(existing)
                        }) {
                            anyhow::bail!(
                                "alias '{}' already exists for {}",
                                candidate.name,
                                candidate.shell
                            );
                        }
                        Ok(())
                    });
                    match validation {
                        Ok(()) => {
                            editor.phase = EditorPhase::Validated;
                            editor.error = None;
                            self.message = "Valid. Press Enter again to review the change.".into();
                        }
                        Err(error) => {
                            editor.error = Some(error.to_string());
                            self.message = error.to_string();
                        }
                    }
                }
                KeyCode::Enter if editor.phase == EditorPhase::Validated => {
                    transition = Some(Screen::Preview(editor.as_change(self.shell)));
                }
                _ => {}
            },
            Screen::Preview(change) => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    commit_change = Some(change.clone());
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    transition = Some(Screen::List);
                    self.message = "Change discarded".into();
                }
                _ => {}
            },
        }
        if let Some(change) = commit_change {
            if let Err(error) = self.commit(change) {
                self.message = format!("Could not save change: {error:#}");
            } else {
                transition = Some(Screen::List);
            }
        }
        if let Some(next) = transition {
            self.screen = next;
            self.selected = self
                .selected
                .min(self.visible_aliases().len().saturating_sub(1));
        }
        quit
    }

    fn commit(&mut self, change: Change) -> Result<()> {
        let mut updated = self.aliases.clone();
        let mut suppressed = self.suppressed.clone();
        match change {
            Change::Create(alias) => {
                suppressed.retain(|key| !key.matches(&alias));
                updated.push(alias);
            }
            Change::Update { before, after } => {
                if let Some(index) = updated
                    .iter()
                    .position(|alias| alias.shell == before.shell && alias.name == before.name)
                {
                    updated[index] = after.clone();
                } else {
                    updated.push(after.clone());
                }
                if before.name != after.name
                    && self.discovered.iter().any(|item| {
                        item.alias.shell == before.shell && item.alias.name == before.name
                    })
                {
                    ensure_suppressed(&mut suppressed, AliasKey::new(before.shell, before.name));
                }
                suppressed.retain(|key| !key.matches(&after));
            }
            Change::Delete(alias) => {
                updated.retain(|candidate| {
                    candidate.shell != alias.shell || candidate.name != alias.name
                });
                if self
                    .discovered
                    .iter()
                    .any(|item| item.alias.shell == alias.shell && item.alias.name == alias.name)
                {
                    ensure_suppressed(&mut suppressed, AliasKey::new(alias.shell, alias.name));
                }
            }
        }
        persist_alias_state(&self.config_dir, &updated, &suppressed)?;
        self.aliases = updated;
        self.suppressed = suppressed;
        self.message = "Saved. After --setup, this shell refreshes aliases when you exit.".into();
        Ok(())
    }
}

fn ensure_suppressed(suppressed: &mut Vec<AliasKey>, key: AliasKey) {
    if !suppressed.contains(&key) {
        suppressed.push(key);
    }
}

fn main() -> Result<()> {
    let options = parse_options()?;
    let profile = options
        .profile
        .clone()
        .map(Ok)
        .unwrap_or_else(|| default_profile(options.shell))?;
    let store_path = alias_manager::aliases_store_path(&options.config_dir);
    let (aliases, suppressed) = load_alias_state(&store_path)?;

    if options.setup {
        return run_setup(
            options.shell,
            &options.config_dir,
            &profile,
            &aliases,
            &suppressed,
        );
    }

    persist_alias_state(&options.config_dir, &aliases, &suppressed)?;
    let discovered = discover_aliases(options.shell, &profile);
    run_tui(
        options.shell,
        options.config_dir,
        aliases,
        discovered,
        suppressed,
    )
}

fn parse_options() -> Result<Options> {
    let mut shell = default_shell();
    let mut config_dir = default_config_dir()?;
    let mut profile = None;
    let mut setup = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--setup" => setup = true,
            "--shell" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow!("--shell needs a value"))?;
                shell = Shell::from_str(&value)?;
            }
            "--config-dir" => {
                config_dir = PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow!("--config-dir needs a path"))?,
                );
            }
            "--profile" => {
                profile = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow!("--profile needs a path"))?,
                ));
            }
            other => bail!("unknown argument '{other}'; use --help for usage"),
        }
    }
    Ok(Options {
        shell,
        config_dir,
        profile,
        setup,
    })
}

fn print_help() {
    println!(
        "Alias Manager\n\nUsage:\n  alias-manager [--shell bash|zsh] [--config-dir PATH] [--profile PATH]\n  alias-manager --setup [--shell bash|zsh] [--config-dir PATH] [--profile PATH]\n  alias-manager --help\n\n--setup previews and, after y confirmation, adds the managed source block to the shell profile."
    );
}

fn run_setup(
    shell: Shell,
    config_dir: &Path,
    profile: &Path,
    aliases: &[Alias],
    suppressed: &[AliasKey],
) -> Result<()> {
    persist_alias_state(config_dir, aliases, suppressed)?;
    let update = prepare_profile_update(profile, config_dir, shell)?;
    println!("Shell setup for {shell}: {}", update.profile.display());
    if !update.changed() {
        println!("The managed alias-manager section is already up to date.");
        return Ok(());
    }
    println!(
        "\nThe following managed block will be added or refreshed:\n\n{}",
        update.snippet
    );
    print!("Apply this profile change and create a backup first? [y/N] ");
    io::stdout().flush().context("flushing prompt")?;
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .context("reading confirmation")?;
    if !matches!(answer.trim(), "y" | "Y") {
        println!("No profile changes made.");
        return Ok(());
    }
    match apply_profile_update(&update)? {
        Some(backup) => println!("Profile updated. Backup: {}", backup.display()),
        None => println!("Profile updated."),
    }
    println!("Open a new shell or reload the profile to activate aliases.");
    Ok(())
}

fn run_tui(
    shell: Shell,
    config_dir: PathBuf,
    aliases: Vec<Alias>,
    discovered: Vec<DiscoveredAlias>,
    suppressed: Vec<AliasKey>,
) -> Result<()> {
    enable_raw_mode().context("enabling terminal raw mode")?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        return Err(error).context("entering alternate screen");
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            return Err(error).context("creating terminal interface");
        }
    };
    let result = terminal
        .hide_cursor()
        .context("hiding terminal cursor")
        .and_then(|()| {
            run_app(
                &mut terminal,
                &mut App::with_discovered(shell, config_dir, aliases, discovered, suppressed),
            )
        });
    let raw_mode_result = disable_raw_mode().context("restoring terminal mode");
    let screen_result =
        execute!(terminal.backend_mut(), LeaveAlternateScreen).context("leaving alternate screen");
    let cursor_result = terminal.show_cursor().context("restoring terminal cursor");
    result
        .and(raw_mode_result)
        .and(screen_result)
        .and(cursor_result)
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key) {
                    return Ok(());
                }
            }
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.area());
    let rows = app.visible_rows();
    let items: Vec<ListItem<'_>> = if rows.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            if app.search.is_empty() {
                "No aliases yet — press a to add one."
            } else {
                "No aliases match this search."
            },
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        rows.iter()
            .map(|row| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<18}", row.alias.name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<12}", row.origin),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(&row.alias.command),
                ]))
            })
            .collect()
    };
    let title = format!(" Alias Manager · {} · {} aliases ", app.shell, rows.len());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(app.selected.min(rows.len() - 1)));
    }
    frame.render_stateful_widget(list, layout[0], &mut state);

    frame.render_widget(
        Paragraph::new(key_hints(app)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Key bindings "),
        ),
        layout[1],
    );
    frame.render_widget(
        Paragraph::new(status_text(app))
            .block(Block::default().borders(Borders::ALL).title(" Status ")),
        layout[2],
    );

    match &app.screen {
        Screen::List => {}
        Screen::Editor(editor) => draw_editor(frame, app.shell, editor),
        Screen::Preview(change) => draw_preview(frame, change),
    }
}

fn key_hints(app: &App) -> &'static str {
    match &app.screen {
        Screen::List if app.searching => {
            "Type to filter · Enter finish search · Esc clear · Backspace erase"
        }
        Screen::List if !app.search.is_empty() => {
            "↑/↓ select · Enter edit · a add · d delete · / search · Esc clear · q quit"
        }
        Screen::List => "↑/↓ select · / search · a add · Enter edit · d delete · q quit",
        Screen::Editor(editor) if editor.phase == EditorPhase::Editing => {
            "Tab switch field · ←/→ cursor · Backspace erase · Enter validate · Esc cancel"
        }
        Screen::Editor(_) => "Enter preview · Esc cancel",
        Screen::Preview(_) => "y confirm · n or Esc discard",
    }
}

fn status_text(app: &App) -> String {
    if app.searching {
        format!("Searching: {}▏", app.search)
    } else if !app.search.is_empty() {
        format!("Filter: {}", app.search)
    } else {
        app.message.clone()
    }
}

fn draw_editor(frame: &mut Frame<'_>, shell: Shell, editor: &Editor) {
    let area = centered_rect(76, 42, frame.area());
    let active_name = if editor.field == Field::Name {
        "› "
    } else {
        "  "
    };
    let active_command = if editor.field == Field::Command {
        "› "
    } else {
        "  "
    };
    let name = editor.display_value(Field::Name);
    let command = editor.display_value(Field::Command);
    let phase = if editor.phase == EditorPhase::Editing {
        "Type values, Tab switches fields"
    } else {
        "Validated · press Enter to open the before/after preview"
    };
    let lines = vec![
        Line::from(format!("Target shell: {shell}")),
        Line::from(""),
        Line::from(format!("{active_name}Name:    {name}")),
        Line::from(format!("{active_command}Command: {command}")),
        Line::from(""),
        Line::from(phase),
        Line::from(Span::styled(
            editor.error.as_deref().unwrap_or(""),
            Style::default().fg(Color::Red),
        )),
        Line::from("←/→ edit · Home/End move to edge · Enter validates/previews · Esc discards"),
    ];
    let title = if editor.original.is_some() {
        " Edit alias "
    } else {
        " Add alias "
    };
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_preview(frame: &mut Frame<'_>, change: &Change) {
    let area = centered_rect(82, 54, frame.area());
    let lines = match change {
        Change::Create(alias) => vec![
            Line::from(Span::styled(
                "CREATE",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("{} ({})", alias.name, alias.shell)),
            Line::from(format!("Command: {}", alias.command)),
        ],
        Change::Update { before, after } => vec![
            Line::from(Span::styled(
                "UPDATE",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("Before · {} ({})", before.name, before.shell)),
            Line::from(format!("Command: {}", before.command)),
            Line::from(""),
            Line::from(format!("After · {} ({})", after.name, after.shell)),
            Line::from(format!("Command: {}", after.command)),
        ],
        Change::Delete(alias) => vec![
            Line::from(Span::styled(
                "DELETE",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("{} ({})", alias.name, alias.shell)),
            Line::from(format!("Command: {}", alias.command)),
        ],
    };
    let mut content = lines;
    content.push(Line::from(""));
    content.push(Line::from(
        "Press y to save this change · n or Esc to discard",
    ));
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Review change "),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height) / 2),
            Constraint::Percentage(height),
            Constraint::Percentage((100 - height) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width) / 2),
            Constraint::Percentage(width),
            Constraint::Percentage((100 - width) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use alias_manager::{load_aliases, persist_aliases};
    use crossterm::event::KeyEventKind;
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Press)
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
    }

    fn new_app(shell: Shell, config_dir: PathBuf, aliases: Vec<Alias>) -> App {
        App::with_discovered(shell, config_dir, aliases, Vec::new(), Vec::new())
    }

    #[test]
    fn create_requires_validation_preview_and_explicit_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = new_app(Shell::Bash, directory.path().to_path_buf(), Vec::new());
        app.handle_key(key(KeyCode::Char('a')));
        type_text(&mut app, "ll");
        app.handle_key(key(KeyCode::Tab));
        type_text(&mut app, "ls -la");

        app.handle_key(key(KeyCode::Enter));
        assert!(
            matches!(&app.screen, Screen::Editor(editor) if editor.phase == EditorPhase::Validated)
        );
        assert!(app.aliases.is_empty());

        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(&app.screen, Screen::Preview(Change::Create(_))));
        assert!(app.aliases.is_empty());

        app.handle_key(key(KeyCode::Char('y')));
        assert!(matches!(app.screen, Screen::List));
        assert_eq!(app.aliases.len(), 1);
        assert_eq!(
            load_aliases(&alias_manager::aliases_store_path(directory.path())).unwrap(),
            app.aliases
        );
    }

    #[test]
    fn rejecting_preview_keeps_saved_data_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = new_app(Shell::Bash, directory.path().to_path_buf(), Vec::new());
        app.handle_key(key(KeyCode::Char('a')));
        type_text(&mut app, "ll");
        app.handle_key(key(KeyCode::Tab));
        type_text(&mut app, "ls");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('n')));

        assert!(matches!(app.screen, Screen::List));
        assert!(app.aliases.is_empty());
        assert!(
            load_aliases(&alias_manager::aliases_store_path(directory.path()))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn search_filters_and_edit_delete_follow_preview_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let original = Alias {
            shell: Shell::Bash,
            name: "ll".into(),
            command: "ls".into(),
        };
        let other_shell_alias = Alias {
            shell: Shell::Zsh,
            name: "ll".into(),
            command: "ls -G".into(),
        };
        let seeded = vec![original.clone(), other_shell_alias];
        persist_aliases(directory.path(), &seeded).unwrap();
        let mut app = new_app(Shell::Bash, directory.path().to_path_buf(), seeded);

        app.handle_key(key(KeyCode::Char('/')));
        type_text(&mut app, "ls");
        assert_eq!(app.visible_aliases(), vec![original.clone()]);
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(&app.screen, Screen::Editor(_)));

        app.handle_key(key(KeyCode::Tab));
        app.handle_key(key(KeyCode::Backspace));
        app.handle_key(key(KeyCode::Backspace));
        type_text(&mut app, "ls -la");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));
        assert!(matches!(
            &app.screen,
            Screen::Preview(Change::Update { .. })
        ));
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.aliases[0], original);

        app.handle_key(key(KeyCode::Char('d')));
        assert!(matches!(&app.screen, Screen::Preview(Change::Delete(_))));
        app.handle_key(key(KeyCode::Char('n')));
        assert_eq!(app.aliases[0].name, "ll");

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert!(app.aliases.iter().all(|alias| alias.shell != Shell::Bash));
        assert_eq!(
            load_aliases(&alias_manager::aliases_store_path(directory.path())).unwrap(),
            app.aliases
        );
    }

    #[test]
    fn discovered_aliases_can_be_overridden_and_suppressed_without_editing_profiles() {
        let directory = tempfile::tempdir().unwrap();
        let existing = Alias {
            shell: Shell::Bash,
            name: "g".into(),
            command: "git".into(),
        };
        let discovered = vec![DiscoveredAlias {
            alias: existing.clone(),
            source: PathBuf::from("/etc/bashrc"),
            line: 12,
        }];
        let mut app = App::with_discovered(
            Shell::Bash,
            directory.path().to_path_buf(),
            Vec::new(),
            discovered,
            Vec::new(),
        );
        assert_eq!(app.visible_aliases(), vec![existing.clone()]);

        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Tab));
        type_text(&mut app, " status");
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(app.visible_aliases()[0].command, "git status");

        app.handle_key(key(KeyCode::Char('d')));
        app.handle_key(key(KeyCode::Char('y')));
        assert!(app.visible_aliases().is_empty());
        let (stored, suppressed) =
            load_alias_state(&alias_manager::aliases_store_path(directory.path())).unwrap();
        assert!(stored.is_empty());
        assert_eq!(suppressed, vec![AliasKey::new(Shell::Bash, "g")]);
        assert!(fs::read_to_string(alias_manager::generated_aliases_path(
            directory.path(),
            Shell::Bash,
        ))
        .unwrap()
        .contains("unalias g 2>/dev/null || true"));
    }

    #[test]
    fn status_messages_do_not_replace_contextual_key_hints() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = new_app(Shell::Bash, directory.path().to_path_buf(), Vec::new());
        let list_keys = key_hints(&app).to_owned();

        app.handle_key(key(KeyCode::Char('a')));
        assert!(key_hints(&app).contains("Enter validate"));
        app.handle_key(key(KeyCode::Esc));

        assert_eq!(status_text(&app), "Draft discarded");
        assert_eq!(key_hints(&app), list_keys);

        app.handle_key(key(KeyCode::Char('/')));
        assert!(key_hints(&app).contains("Type to filter"));
        assert!(status_text(&app).starts_with("Searching:"));
    }
}
