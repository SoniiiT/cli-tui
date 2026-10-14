# Venv Runner

A fullscreen TUI for Python projects. Open it in a project directory:

- If the project has **no virtual environment**, it opens a form where you choose the folder name (e.g. `.venv-abc`), the Python version, and which dependencies to install, then creates the venv.
- If a **venv already exists**, it finds it and you just press **Start**. Venv Runner leaves the TUI and runs `<venv>/bin/python main.py` directly in your terminal, so you see all output, can answer `input()` prompts, and can debug with `pdb`/`breakpoint()` as usual. Afterwards you return to the TUI.

## Prerequisites

- Python 3 with the `venv` and `ensurepip` modules, to create venvs. On Debian/Ubuntu this means the `python3-venv` package. Interpreters without these modules are not offered.
- To build: Rust stable (Cargo).

## Installation and startup

```sh
cargo build --release --manifest-path tools/venv-runner/Cargo.toml
install -m 755 tools/venv-runner/target/release/venv-runner /opt/homebrew/bin/   # or any directory on your PATH
# alternatively: cargo install --path tools/venv-runner
```

Updating an installed copy: close running Venv Runner sessions first, and replace the file with `install` (which installs a new file) or `cargo install`, never with a plain `cp` over the old one. On macOS, overwriting a binary in place while it runs makes the next launch get killed (`Killed: 9`, exit 137) and can kill the running sessions.

Run it in a project, or point it at one:

```sh
cd my-python-project && venv-runner
venv-runner path/to/my-python-project
```

It needs an interactive terminal. `venv-runner --help` and `--version` are available.

## Usage

### Finding venvs

Every direct subfolder of the project that contains a `pyvenv.cfg` is a venv, whatever it is called (`.venv`, `venv`, `.venv-abc`, ...). The list shows each venv's Python version; a venv whose Python executable is gone (for example because the interpreter it was created from was uninstalled) is marked **broken** and cannot be started, but it can be deleted. A project without any venv opens the create form directly.

### Starting

Select a venv and press **Enter** (or `s`). Venv Runner runs the venv's own interpreter with the **run command** as arguments, with the environment an activated venv would have (`VIRTUAL_ENV` set, the venv's `bin` first in `PATH`, `PYTHONHOME` removed). The run command defaults to `main.py` (or `app.py`, `run.py`, `src/main.py`, `app/main.py`, `__main__.py` if that is what the project has). Press **e** to change it; it is passed to Python with shell-style quoting, so all of these work:

```text
main.py
main.py --debug --name "Jane Doe"
-m uvicorn app:app --reload
```

While the program runs, the terminal is yours: Ctrl+C goes to your Python program (not to Venv Runner), `input()` and debuggers work. When it ends you see its exit code or signal and a prompt: **Enter** returns to the TUI, **q** quits.

### Creating

Press **n** (or select *+ New virtual environment…*) to open the form:

| Field | What it does |
| --- | --- |
| Name | Folder name of the venv. Defaults to `.venv`, or `.venv-2`, ... if that is taken. Letters, digits, `.`, `_`, `-` only. |
| Python | The interpreter to create the venv from, chosen with `←`/`→`. Found via `PATH`, pyenv (`~/.pyenv/versions`, or `$PYENV_ROOT`) and uv-managed Pythons (`~/.local/share/uv/python`, or `$UV_PYTHON_INSTALL_DIR`); each candidate is started once to confirm it works. Newest first. Preselected: the version from `.python-version`, else what `python3` runs (marked *default*). `requires-python` from `pyproject.toml` is shown as a hint. |
| Dependencies | Installs into the new venv: a `requirements*.txt` file (`pip install -r`), an installable `pyproject.toml` or `setup.py` (`pip install -e .`), or nothing. The first available source is preselected. |

**Enter** creates the venv. This also happens in the real terminal (`python -m venv NAME`, then `NAME/bin/python -m pip install ...`), so you see pip's output and any error exactly as in a shell.

### Updating

Press **u** on a venv to bring it up to date, in place. This is the creation recipe applied to the existing venv, so the defaults give you what a fresh venv would get:

| Field | What it does |
| --- | --- |
| Dependencies | The source to install again: a `requirements*.txt`, an installable `pyproject.toml`/`setup.py` (`pip install -e .`), or none. The first detected source is preselected. |
| Upgrade pip | `python -m pip install --upgrade pip`. On by default. |
| Packages | *upgrade to the newest versions allowed* (default) runs `pip install --upgrade --upgrade-strategy eager`, which also moves dependencies of your dependencies forward, like a fresh install would resolve them. *install what is missing only* leaves installed packages alone. |

The dialog lists the exact commands that will run and updates the list as you change the choices. **Enter** runs them in the real terminal, so you see pip's output and errors as in a shell; afterwards you return to the TUI. `←`/`→` or Space change a choice, Esc cancels.

Updating does not remove packages that are no longer required (pip never does), and it does not change the venv's Python version. For a clean slate, or to move to another Python, delete the venv (`d`) and create a new one (`n`). A broken venv cannot be updated because pip cannot run in it; the TUI says so and points to delete + new.

### Deleting

Select a venv and press **d** (or `Delete`). A confirmation shows the folder, its Python version and how many files and how much disk space it holds. Only **y** deletes; Enter, `n` and Esc keep the venv. Broken venvs can be deleted like any other, which is the way to get rid of one before creating a replacement.

Deleting is the only destructive action, so it is deliberately narrow: it removes just that one folder, only if it is a direct subfolder of the project that still contains `pyvenv.cfg`, and never anything it points to. If the venv folder is a **symlink** to a venv elsewhere, only the link is removed and the dialog says so.

### Key bindings

| Key | Action |
| --- | --- |
| `↑`/`↓`, `j`/`k` | Select venv / new-venv row |
| `Enter`, `s` | Start the selected venv (on the new-venv row: open the form) |
| `n` | New venv |
| `u` | Update the selected venv: upgrade pip and (re)install the dependencies |
| `d`, `Delete` | Delete the selected venv (asks first; `y` confirms) |
| `e` | Edit the run command |
| `r` | Rescan the project |
| `q`, `Esc`, `Ctrl+C` | Quit |
| Form: `Tab`/`↑`/`↓` | Next / previous field |
| Form: `←`/`→` | Change Python / dependency choice |
| Form: `Enter` / `Esc` | Create / cancel |
| Text fields: `Ctrl+U` | Clear the field |

## Failure cases and limitations

- Venvs are created with `python -m venv`, never with `uv`/`virtualenv`; uv-managed Pythons are offered as interpreters, though.
- If creating the venv itself fails, the half-built folder is removed. If only the dependency install fails, the venv is kept and the status line says so, so you can fix `requirements.txt` and install manually or create it again under another name.
- There is no one-step "recreate": delete the venv (`d`), then create a new one (`n`). Update (`u`) works in place and never removes packages or changes the Python version.
- The run command is not a shell line: quoting is understood, but pipes, globs, `$VAR` expansion and `&&` are not.
- The run command is not remembered between sessions. Only direct subfolders of the project are scanned for venvs.
- `.python-version` and `requires-python` only preselect or hint; they are not enforced.
- On macOS, `/usr/bin/python3` is not probed when the Xcode Command Line Tools are missing, because running that stub would open the installer.
- Windows is not verified: venv paths (`Scripts\python.exe`) are handled in the code, but Ctrl+C forwarding to the child is not implemented there.

## Tests

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

The tests include ones that create a real venv from the Python found on the machine and run code in it, so a working `python3` is expected.

## Platform status

| Platform | Status | Evidence |
| --- | --- | --- |
| macOS | `supported` | macOS 27.0.0 (Darwin) arm64, tested 2026-10-12. `cargo test` (93 tests), Clippy, `cargo fmt --check` and release build native; the release binary's core path (detect, create, update, start with `input()`/`pdb`, Ctrl+C, exit codes, delete with confirmation, paths with spaces) was exercised in a real pty. |
| Linux | `unverified` | Not tested yet. |
| Windows | `unverified` | Not tested yet; see limitations above. |

Platform status requirements are in [`../../docs/platform-support.md`](../../docs/platform-support.md).
