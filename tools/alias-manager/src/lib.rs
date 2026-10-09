use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

const STORE_VERSION: u32 = 1;
const START_MARKER: &[u8] = b"# >>> alias-manager >>>";
const END_MARKER: &[u8] = b"# <<< alias-manager <<<";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    pub fn as_str(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
        }
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Shell {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            _ => bail!("unsupported shell '{value}'; choose bash or zsh"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Alias {
    pub shell: Shell,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AliasStore {
    version: u32,
    aliases: Vec<Alias>,
    #[serde(default)]
    suppressed: Vec<AliasKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AliasKey {
    pub shell: Shell,
    pub name: String,
}

impl AliasKey {
    pub fn new(shell: Shell, name: impl Into<String>) -> Self {
        Self {
            shell,
            name: name.into(),
        }
    }

    pub fn matches(&self, alias: &Alias) -> bool {
        self.shell == alias.shell && self.name == alias.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredAlias {
    pub alias: Alias,
    pub source: PathBuf,
    pub line: usize,
}

#[derive(Debug)]
pub struct ProfileUpdate {
    pub profile: PathBuf,
    pub snippet: String,
    pub original: Vec<u8>,
    pub updated: Vec<u8>,
    existed: bool,
    permissions: Option<fs::Permissions>,
}

impl ProfileUpdate {
    pub fn changed(&self) -> bool {
        self.original != self.updated
    }
}

pub fn default_shell() -> Shell {
    match env::var("SHELL") {
        Ok(shell) if shell.ends_with("zsh") => Shell::Zsh,
        _ => Shell::Bash,
    }
}

pub fn default_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return Ok(home_dir()?.join("Library/Application Support/alias-manager"));
    }

    #[cfg(target_os = "windows")]
    {
        let appdata = env::var_os("APPDATA").map(PathBuf::from).ok_or_else(|| {
            anyhow!("APPDATA is not set; pass --config-dir with an explicit path")
        })?;
        return Ok(appdata.join("alias-manager"));
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(path).join("alias-manager"));
        }
        Ok(home_dir()?.join(".config/alias-manager"))
    }
}

pub fn default_profile(shell: Shell) -> Result<PathBuf> {
    let home = home_dir()?;
    #[cfg(target_os = "macos")]
    let profile = match shell {
        Shell::Bash => ".bash_profile",
        Shell::Zsh => ".zshrc",
    };
    #[cfg(not(target_os = "macos"))]
    let profile = match shell {
        Shell::Bash => ".bashrc",
        Shell::Zsh => ".zshrc",
    };
    Ok(home.join(profile))
}

pub fn aliases_store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("aliases.json")
}

pub fn generated_aliases_path(config_dir: &Path, shell: Shell) -> PathBuf {
    config_dir.join(format!("{}-aliases.sh", shell.as_str()))
}

pub fn load_aliases(path: &Path) -> Result<Vec<Alias>> {
    load_alias_state(path).map(|(aliases, _)| aliases)
}

pub fn load_alias_state(path: &Path) -> Result<(Vec<Alias>, Vec<AliasKey>)> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()))
        }
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    let store: AliasStore = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing alias data in {}", path.display()))?;
    if store.version != STORE_VERSION {
        bail!(
            "unsupported alias data version {} in {} (supported: {})",
            store.version,
            path.display(),
            STORE_VERSION
        );
    }
    validate_aliases(&store.aliases)?;
    validate_alias_keys(&store.suppressed)?;
    Ok((store.aliases, store.suppressed))
}

pub fn persist_aliases(config_dir: &Path, aliases: &[Alias]) -> Result<()> {
    persist_alias_state(config_dir, aliases, &[])
}

pub fn persist_alias_state(
    config_dir: &Path,
    aliases: &[Alias],
    suppressed: &[AliasKey],
) -> Result<()> {
    validate_aliases(aliases)?;
    validate_alias_keys(suppressed)?;
    fs::create_dir_all(config_dir)
        .with_context(|| format!("creating config directory {}", config_dir.display()))?;

    let mut sorted = aliases.to_vec();
    sorted.sort_by(|left, right| {
        left.shell
            .cmp(&right.shell)
            .then_with(|| left.name.cmp(&right.name))
    });
    let store = AliasStore {
        version: STORE_VERSION,
        aliases: sorted.clone(),
        suppressed: suppressed.to_vec(),
    };
    for shell in [Shell::Bash, Shell::Zsh] {
        let rendered = render_alias_state(shell, &sorted, suppressed)?;
        atomic_write(
            &generated_aliases_path(config_dir, shell),
            rendered.as_bytes(),
        )?;
    }
    let json = serde_json::to_vec_pretty(&store).context("serializing alias data")?;
    // The JSON store is canonical; publish it last so a failed rendered-file write
    // never reports success or replaces the saved alias collection.
    atomic_write(&aliases_store_path(config_dir), &json)?;
    Ok(())
}

pub fn validate_alias(alias: &Alias) -> Result<()> {
    validate_alias_name(&alias.name)?;
    if alias.command.trim().is_empty() {
        bail!("alias command cannot be empty");
    }
    Ok(())
}

fn validate_alias_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-' | b'+'))
    {
        bail!("alias name may contain only ASCII letters, digits, _, ., - and +");
    }
    Ok(())
}

fn validate_alias_keys(keys: &[AliasKey]) -> Result<()> {
    for key in keys {
        validate_alias_name(&key.name)
            .with_context(|| format!("invalid suppressed alias '{}':", key.name))?;
    }
    for (index, key) in keys.iter().enumerate() {
        if keys[index + 1..].iter().any(|other| other == key) {
            bail!("alias '{}' is suppressed more than once", key.name);
        }
    }
    Ok(())
}

pub fn validate_aliases(aliases: &[Alias]) -> Result<()> {
    for alias in aliases {
        validate_alias(alias).with_context(|| format!("invalid alias '{}':", alias.name))?;
    }
    for (index, alias) in aliases.iter().enumerate() {
        if aliases[index + 1..]
            .iter()
            .any(|other| other.shell == alias.shell && other.name == alias.name)
        {
            bail!("alias '{}' already exists for {}", alias.name, alias.shell);
        }
    }
    Ok(())
}

pub fn render_aliases(shell: Shell, aliases: &[Alias]) -> Result<String> {
    render_alias_state(shell, aliases, &[])
}

pub fn render_alias_state(
    shell: Shell,
    aliases: &[Alias],
    suppressed: &[AliasKey],
) -> Result<String> {
    validate_aliases(aliases)?;
    validate_alias_keys(suppressed)?;
    let mut selected: Vec<_> = aliases
        .iter()
        .filter(|alias| alias.shell == shell)
        .collect();
    selected.sort_by(|left, right| left.name.cmp(&right.name));
    let mut output = String::from("# Generated by alias-manager. Manual edits are overwritten.\n");
    let mut hidden: Vec<_> = suppressed.iter().filter(|key| key.shell == shell).collect();
    hidden.sort_by(|left, right| left.name.cmp(&right.name));
    for key in hidden {
        output.push_str("unalias ");
        output.push_str(&key.name);
        output.push_str(" 2>/dev/null || true\n");
    }
    for alias in selected {
        output.push_str("alias ");
        output.push_str(&alias.name);
        output.push('=');
        output.push_str(&shell_quote(&alias.command));
        output.push('\n');
    }
    Ok(output)
}

/// Read literal alias declarations and recursively follow static `source`/`.` paths.
/// Shell startup files are parsed as text; they are never executed.
pub fn discover_aliases(shell: Shell, profile: &Path) -> Vec<DiscoveredAlias> {
    let mut discovered = BTreeMap::new();
    let mut visited = HashSet::new();
    scan_profile(shell, profile, &mut visited, &mut discovered);
    discovered.into_values().collect()
}

fn scan_profile(
    shell: Shell,
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    discovered: &mut BTreeMap<String, DiscoveredAlias>,
) {
    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(resolved.clone()) {
        return;
    }
    match fs::metadata(&resolved) {
        Ok(metadata) if metadata.is_file() && metadata.len() <= 4 * 1024 * 1024 => {}
        _ => return,
    }
    let contents = match fs::read_to_string(&resolved) {
        Ok(contents) => contents,
        Err(_) => return,
    };

    for (line_index, line) in contents.lines().enumerate() {
        for source in source_paths(line) {
            scan_profile(shell, &source, visited, discovered);
        }
        for (name, command) in parse_alias_declarations(line) {
            let alias = Alias {
                shell,
                name: name.clone(),
                command,
            };
            if validate_alias(&alias).is_ok() {
                discovered.insert(
                    name,
                    DiscoveredAlias {
                        alias,
                        source: resolved.clone(),
                        line: line_index + 1,
                    },
                );
            }
        }
    }
}

fn source_paths(line: &str) -> Vec<PathBuf> {
    let words = shell_words(line);
    let mut paths = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let command_position =
            index == 0 || matches!(words[index - 1].as_str(), ";" | "&&" | "||" | "then" | "do");
        if command_position && (word == "source" || word == ".") && index + 1 < words.len() {
            if let Some(path) = expand_source_path(&words[index + 1]) {
                paths.push(path);
            }
        }
    }
    paths
}

fn expand_source_path(value: &str) -> Option<PathBuf> {
    if value.is_empty() || value.contains("$(") || value.contains('`') {
        return None;
    }
    let mut expanded = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            expanded.push(ch);
            continue;
        }
        let (key, default) = if chars.peek() == Some(&'{') {
            chars.next();
            let mut expression = String::new();
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
                expression.push(next);
            }
            let (key, default) = expression
                .split_once(":=")
                .map_or((expression.as_str(), None), |(key, default)| {
                    (key, Some(default))
                });
            (key.to_owned(), default.map(str::to_owned))
        } else {
            let mut key = String::new();
            while chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
            {
                key.push(chars.next().unwrap());
            }
            if key.is_empty() {
                expanded.push('$');
                continue;
            }
            (key, None)
        };
        let value = env::var_os(&key)
            .map(|value| value.to_string_lossy().into_owned())
            .or(default)?;
        expanded.push_str(&value);
    }

    if let Some(rest) = expanded.strip_prefix("~/") {
        expanded = home_dir().ok()?.join(rest).to_string_lossy().into_owned();
    }
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        Some(path)
    } else {
        env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

fn parse_alias_declarations(line: &str) -> Vec<(String, String)> {
    let trimmed = line.trim_start();
    let rest = match trimmed.strip_prefix("alias") {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest,
        _ => return Vec::new(),
    };
    shell_words(rest)
        .into_iter()
        .take_while(|word| !matches!(word.as_str(), ";" | "&&" | "||"))
        .filter_map(|word| {
            if word == "--" || word == ";" {
                return None;
            }
            let (name, command) = word.split_once('=')?;
            validate_alias_name(name).ok()?;
            Some((name.to_owned(), command.to_owned()))
        })
        .collect()
}

fn shell_words(line: &str) -> Vec<String> {
    #[derive(Clone, Copy)]
    enum Quote {
        None,
        Single,
        Double,
    }

    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote = Quote::None;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                } else {
                    word.push(ch);
                }
            }
            Quote::Double => match ch {
                '"' => quote = Quote::None,
                '\\' => match chars.next() {
                    Some(next @ ('$' | '`' | '"' | '\\')) => word.push(next),
                    Some('\n') => {}
                    Some(next) => {
                        word.push('\\');
                        word.push(next);
                    }
                    None => word.push('\\'),
                },
                _ => word.push(ch),
            },
            Quote::None => match ch {
                '\'' => {
                    quote = Quote::Single;
                    started = true;
                }
                '"' => {
                    quote = Quote::Double;
                    started = true;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        word.push(next);
                    }
                    started = true;
                }
                '#' if !started => break,
                ';' => {
                    if started {
                        words.push(std::mem::take(&mut word));
                        started = false;
                    }
                    words.push(";".into());
                }
                ch if ch.is_whitespace() => {
                    if started {
                        words.push(std::mem::take(&mut word));
                        started = false;
                    }
                }
                _ => {
                    word.push(ch);
                    started = true;
                }
            },
        }
    }
    if started {
        words.push(word);
    }
    words
}

pub fn prepare_profile_update(
    profile: &Path,
    config_dir: &Path,
    shell: Shell,
) -> Result<ProfileUpdate> {
    let metadata = match fs::symlink_metadata(profile) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("checking {}", profile.display())),
    };
    if metadata
        .as_ref()
        .is_some_and(|metadata| metadata.file_type().is_symlink())
    {
        bail!(
            "refusing to replace symbolic-link profile {}",
            profile.display()
        );
    }
    let (original, existed) = if metadata.is_some() {
        (
            fs::read(profile).with_context(|| format!("reading {}", profile.display()))?,
            true,
        )
    } else {
        (Vec::new(), false)
    };
    let alias_path = generated_aliases_path(config_dir, shell);
    let snippet = integration_snippet(&alias_path);
    let updated = update_profile_bytes(&original, snippet.as_bytes())?;
    Ok(ProfileUpdate {
        profile: profile.to_path_buf(),
        snippet,
        original,
        updated,
        existed,
        permissions: metadata.map(|metadata| metadata.permissions()),
    })
}

pub fn apply_profile_update(update: &ProfileUpdate) -> Result<Option<PathBuf>> {
    if !update.changed() {
        return Ok(None);
    }
    let parent = update
        .profile
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("creating profile directory {}", parent.display()))?;

    let backup = if update.existed {
        let backup = backup_path(&update.profile)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&backup)
            .with_context(|| format!("creating profile backup {}", backup.display()))?;
        if let Some(permissions) = update.permissions.clone() {
            fs::set_permissions(&backup, permissions)
                .with_context(|| format!("preserving permissions for {}", backup.display()))?;
        }
        file.write_all(&update.original)
            .and_then(|()| file.sync_all())
            .with_context(|| format!("writing profile backup {}", backup.display()))?;
        Some(backup)
    } else {
        None
    };
    atomic_write(&update.profile, &update.updated)?;
    Ok(backup)
}

pub fn integration_snippet(alias_file: &Path) -> String {
    format!(
        "{start_marker}\n[ -f {alias_file} ] && . {alias_file}\nalias-manager() {{\n  command alias-manager \"$@\"\n  local alias_manager_status=$?\n  if [ \"$alias_manager_status\" -eq 0 ]; then\n    . {alias_file}\n  fi\n  return \"$alias_manager_status\"\n}}\n{end_marker}\n",
        start_marker = String::from_utf8_lossy(START_MARKER),
        alias_file = shell_quote(&alias_file.to_string_lossy()),
        end_marker = String::from_utf8_lossy(END_MARKER)
    )
}

fn update_profile_bytes(original: &[u8], snippet: &[u8]) -> Result<Vec<u8>> {
    let starts = find_all(original, START_MARKER);
    let ends = find_all(original, END_MARKER);
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let mut updated = original.to_vec();
            if !updated.is_empty() && !updated.ends_with(b"\n") {
                updated.push(b'\n');
            }
            updated.extend_from_slice(snippet);
            Ok(updated)
        }
        ([start], [end]) if start < end => {
            let mut end_exclusive = end + END_MARKER.len();
            if original.get(end_exclusive) == Some(&b'\r') {
                end_exclusive += 1;
            }
            if original.get(end_exclusive) == Some(&b'\n') {
                end_exclusive += 1;
            }
            let mut updated = Vec::with_capacity(original.len() + snippet.len());
            updated.extend_from_slice(&original[..*start]);
            updated.extend_from_slice(snippet);
            updated.extend_from_slice(&original[end_exclusive..]);
            Ok(updated)
        }
        _ => bail!("profile contains incomplete, reversed, or duplicate alias-manager markers"),
    }
}

fn find_all(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > bytes.len() {
        return Vec::new();
    }
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, candidate)| (candidate == needle).then_some(index))
        .collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("could not determine the home directory"))
}

fn backup_path(profile: &Path) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let mut name = profile
        .file_name()
        .ok_or_else(|| anyhow!("profile path has no file name"))?
        .to_os_string();
    name.push(format!(".alias-manager.bak-{timestamp}"));
    Ok(profile.with_file_name(name))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("checking {}", path.display())),
    };
    if metadata
        .as_ref()
        .is_some_and(|metadata| metadata.file_type().is_symlink())
    {
        bail!("refusing to replace symbolic link {}", path.display());
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("creating directory {}", parent.display()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("path has no file name: {}", path.display()))?
        .to_string_lossy();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        counter
    ));
    let permissions = metadata.map(|metadata| metadata.permissions());

    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .with_context(|| format!("creating temporary file {}", temporary.display()))?;
        if let Some(permissions) = permissions {
            fs::set_permissions(&temporary, permissions)
                .with_context(|| format!("preserving permissions for {}", path.display()))?;
        }
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .with_context(|| format!("writing temporary file {}", temporary.display()))?;
        fs::rename(&temporary, path)
            .with_context(|| format!("replacing {} atomically", path.display()))?;
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alias(shell: Shell, name: &str, command: &str) -> Alias {
        Alias {
            shell,
            name: name.to_owned(),
            command: command.to_owned(),
        }
    }

    #[test]
    fn validates_names_commands_and_shell_scoped_duplicates() {
        assert!(validate_alias(&alias(Shell::Bash, "ll", "ls -la")).is_ok());
        assert!(validate_alias(&alias(Shell::Bash, "bad name", "ls")).is_err());
        assert!(validate_alias(&alias(Shell::Bash, "..", "cd ..")).is_ok());
        assert!(validate_alias(&alias(Shell::Bash, "ll", "  ")).is_err());
        assert!(validate_aliases(&[
            alias(Shell::Bash, "ll", "ls"),
            alias(Shell::Zsh, "ll", "ls -a")
        ])
        .is_ok());
        assert!(validate_aliases(&[
            alias(Shell::Bash, "ll", "ls"),
            alias(Shell::Bash, "ll", "ls -a")
        ])
        .is_err());
    }

    #[test]
    fn renders_commands_with_shell_single_quote_escaping() {
        let rendered = render_aliases(
            Shell::Bash,
            &[
                alias(Shell::Bash, "say", "printf '%s\\n' hello"),
                alias(Shell::Zsh, "other", "ignored"),
            ],
        )
        .unwrap();
        assert_eq!(
            rendered,
            "# Generated by alias-manager. Manual edits are overwritten.\nalias say='printf '\\''%s\\n'\\'' hello'\n"
        );
    }

    #[test]
    fn updates_only_the_marked_profile_region_and_is_idempotent() {
        let snippet = "# >>> alias-manager >>>\n. '/tmp/aliases.sh'\n# <<< alias-manager <<<\n";
        let before = b"# user before\nexport EDITOR=vi\n";
        let once = update_profile_bytes(before, snippet.as_bytes()).unwrap();
        let twice = update_profile_bytes(&once, snippet.as_bytes()).unwrap();
        assert_eq!(once, twice);

        let changed = snippet.replace("/tmp/aliases.sh", "/var/tmp/new aliases.sh");
        let updated = update_profile_bytes(&once, changed.as_bytes()).unwrap();
        assert!(updated.starts_with(before));
        assert!(String::from_utf8(updated)
            .unwrap()
            .contains("/var/tmp/new aliases.sh"));
    }

    #[test]
    fn rejects_damaged_or_multiple_profile_markers() {
        assert!(update_profile_bytes(b"# >>> alias-manager >>>\n", b"new\n").is_err());
        assert!(update_profile_bytes(
            b"# >>> alias-manager >>>\n# <<< alias-manager <<<\n# >>> alias-manager >>>\n# <<< alias-manager <<<\n",
            b"new\n"
        )
        .is_err());
    }

    #[test]
    fn persists_versioned_data_and_rendered_shell_files() {
        let directory = tempfile::tempdir().unwrap();
        let aliases = [alias(Shell::Zsh, "g", "git status")];
        let suppressed = [AliasKey::new(Shell::Bash, "..")];
        persist_alias_state(directory.path(), &aliases, &suppressed).unwrap();

        let loaded = load_aliases(&aliases_store_path(directory.path())).unwrap();
        assert_eq!(loaded, aliases);
        let (_, loaded_suppressed) =
            load_alias_state(&aliases_store_path(directory.path())).unwrap();
        assert_eq!(loaded_suppressed, suppressed);
        assert!(
            fs::read_to_string(generated_aliases_path(directory.path(), Shell::Zsh))
                .unwrap()
                .contains("alias g='git status'")
        );
        assert!(
            !fs::read_to_string(generated_aliases_path(directory.path(), Shell::Bash))
                .unwrap()
                .contains("alias g=")
        );
        assert!(
            fs::read_to_string(generated_aliases_path(directory.path(), Shell::Bash))
                .unwrap()
                .contains("unalias .. 2>/dev/null || true")
        );
    }

    #[test]
    fn discovers_static_aliases_through_source_without_running_profile_commands() {
        let directory = tempfile::tempdir().unwrap();
        let profile = directory.path().join(".bashrc");
        let sourced = directory.path().join("user aliases.sh");
        let marker = directory.path().join("must-not-be-created");
        fs::write(&profile, format!("source '{}'\n", sourced.display())).unwrap();
        fs::write(
            &sourced,
            format!(
                "# alias hidden='no'\nalias ..='cd ..'\nalias ll='ls -la'\nalias say=\"printf 'hello world'\"\ntouch '{}'\n",
                marker.display()
            ),
        )
        .unwrap();

        let discovered = discover_aliases(Shell::Bash, &profile);
        assert_eq!(discovered.len(), 3);
        assert!(discovered.iter().any(|item| {
            item.alias.name == ".." && item.alias.command == "cd .." && item.line == 2
        }));
        assert!(discovered.iter().any(|item| {
            item.alias.name == "say" && item.alias.command == "printf 'hello world'"
        }));
        assert!(!marker.exists());
    }

    #[test]
    fn setup_backups_original_profile_and_creates_idempotent_integration() {
        let directory = tempfile::tempdir().unwrap();
        let profile = directory.path().join(".bashrc");
        fs::write(&profile, b"# existing config\n").unwrap();
        let update = prepare_profile_update(&profile, directory.path(), Shell::Bash).unwrap();
        let backup = apply_profile_update(&update).unwrap().unwrap();
        assert_eq!(fs::read(backup).unwrap(), b"# existing config\n");
        assert!(fs::read_to_string(&profile)
            .unwrap()
            .contains("# >>> alias-manager >>>"));
        assert!(fs::read_to_string(&profile)
            .unwrap()
            .contains("command alias-manager \"$@\""));
        assert!(fs::read_to_string(&profile)
            .unwrap()
            .contains("local alias_manager_status=$?"));

        let second = prepare_profile_update(&profile, directory.path(), Shell::Bash).unwrap();
        assert!(!second.changed());
        assert!(apply_profile_update(&second).unwrap().is_none());
    }

    #[test]
    fn rejects_symbolic_link_profiles() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("real-profile");
            let link = directory.path().join(".bashrc");
            fs::write(&target, "# original\n").unwrap();
            symlink(&target, &link).unwrap();
            assert!(prepare_profile_update(&link, directory.path(), Shell::Bash).is_err());
        }
    }
}
