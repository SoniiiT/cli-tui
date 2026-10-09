# Alias Manager

Alias Manager is a standalone fullscreen TUI for discovering and managing Bash and Zsh aliases from shell profiles. Alias names and commands are configurable. Changes are saved only after a before/after preview and explicit confirmation. The application never runs alias commands or shell profiles.

## Prerequisites

- Rust stable and Cargo to build the application.
- An interactive terminal.
- Bash or Zsh to activate managed aliases in a shell.

## Installation and startup

Build and run from `tools/alias-manager`:

```sh
cargo build --release
cargo run --release
```

The executable is written to `target/release/alias-manager`. Bash is selected by default unless `SHELL` ends in `zsh`.

Options:

```text
alias-manager [--shell bash|zsh] [--config-dir PATH] [--profile PATH]
alias-manager --setup [--shell bash|zsh] [--config-dir PATH] [--profile PATH]
alias-manager --help
```

`--config-dir` and `--profile` are useful for tests and custom shell profiles. By default, Linux uses `~/.bashrc` or `~/.zshrc`; macOS uses `~/.bash_profile` or `~/.zshrc`. On TUI startup, the manager reads alias declarations from the selected profile and statically referenced `source`/`.` files.

## Usage

| Key | Action |
| --- | --- |
| `/` | Start live search by name or command |
| `↑` / `↓` | Select an alias |
| `Enter` | Edit the selected alias |
| `a` | Add an alias |
| `d` | Mark the selected alias for deletion |
| `Tab` | Switch between name and command fields |
| `←` / `→`, `Home` / `End` | Move the cursor in the active field |
| `Enter`, then `Enter` again | Validate the draft and open its preview |
| `y` / `n` | Apply / discard the previewed change |
| `Esc` | Close a dialog or clear search |
| `q` | Quit |

In the editor, type at the cursor; `Backspace` deletes the preceding character. Discovered aliases show their source file. Editing creates a managed override; deleting hides the alias through the generated shell file without changing the original profile line. JSON data and rendered shell files are updated atomically after saving.

To enable integration, run `alias-manager --setup`, review the profile change, and confirm it with `y`. The program creates a dated backup before changing the profile and only manages its marked section. In the current shell, load the updated profile once with `source ~/.bashrc` or `source ~/.zshrc`, or open a new terminal. After that, the configured `alias-manager` shell function sources the generated alias file automatically when the TUI exits. Launch the tool through this function, not through the binary path directly.

## Storage and safety

- Linux: `$XDG_CONFIG_HOME/alias-manager` or `~/.config/alias-manager`.
- macOS: `~/Library/Application Support/alias-manager`.
- Aliases are stored in versioned `aliases.json`; generated shell files are `bash-aliases.sh` and `zsh-aliases.sh`.
- Newly created config and shell files use Unix mode `0600`; updates preserve existing permissions.
- Duplicate names within the same shell are rejected. Bash and Zsh may each have an alias with the same name.
- Alias names may contain ASCII letters, digits, `_`, `.`, `-`, and `+`; commands must not be empty.
- Profiles are read as text and never executed. Discovery supports simple static `alias name=value` declarations and static `source`/`.` paths. Dynamically generated aliases and conditional profile behavior are not evaluated; for repeated names, the last declaration read wins. Global aliases in unreferenced system files and shell functions are outside the MVP.
- Profile integration is performed only through `--setup`. Bytes outside the managed section are preserved; the backup is stored next to the profile with the suffix `.alias-manager.bak-<timestamp>`.

## Failure cases and limitations

- The MVP implements Bash and Zsh. Windows/PowerShell and other shells are unsupported.
- Profile changes require interactive confirmation. Symlink profiles and profiles with damaged or duplicate markers are rejected.
- On permission or write errors, the TUI keeps the preview state and displays an error message.
- The Bash core path was installed and tested on Linux. macOS has not been tested natively and remains `unverified`; having macOS-specific paths implemented is not sufficient evidence of support.

## Tests

```sh
cargo test
cargo fmt -- --check
```

## Platform status

| Platform | Status | Evidence |
| --- | --- | --- |
| Linux | `supported` (Bash core path) | Omarchy 4.0.4, Linux 7.2.5-3-omarchy x86_64, tested 2026-10-08. `cargo test` (13 tests), Clippy, release build, and `cargo install` succeeded; profile integration and alias activation were verified. |
| macOS | `unverified` | Bash/Zsh profile paths are implemented, but no native build and core-path test has been run. |
| Windows | `unsupported` | PowerShell and Windows shell integration are not implemented. |

Platform status requirements are in [`../../docs/platform-support.md`](../../docs/platform-support.md).
