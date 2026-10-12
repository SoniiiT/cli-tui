# cli-tui

A collection of standalone CLI and TUI applications for Windows, Linux, and macOS. Each tool addresses its own use case and may use its own language, toolchain, and release strategy. Platform support is determined and verified separately for each tool.

## Tool catalog

| Tool | Purpose | Windows | Linux | macOS |
| --- | --- | --- | --- | --- |
| [Alias Manager](tools/alias-manager/README.md) | Discover and manage Bash/Zsh aliases from shell profiles | Unsupported | Supported | Supported |

Platform statuses and evidence requirements are defined in [`docs/platform-support.md`](docs/platform-support.md). Linux and macOS are both supported for the tested Bash/Zsh core path.

## Repository conventions

- Each application has its own directory under `tools/`. Tools remain independent and are not forced into a shared application or release cycle.
- Each tool directory contains a `README.md` describing its purpose, prerequisites, installation, usage, failure cases, and known platform limitations, plus a `tool.toml` containing its ID, purpose, prerequisites, and status for all three platforms.
- Implementation and tests live in the relevant tool directory. Their layout follows the chosen language and toolchain; shared `src/` or `tests/` directories are not required.
- The catalog in this file must match the values in each tool's `tool.toml`.

Repository structure and an example of an included tool:

```text
README.md
docs/
  platform-support.md
  plans/
tools/
  README.md
  _template/
    README.md
    tool.toml
    src/                 # starter structure; adapt as needed
    tests/               # optional; tests may live beside source code
  <tool-id>/
    README.md
    tool.toml
    <toolchain-specific files and directories>
  alias-manager/
    Cargo.toml
    Cargo.lock
    README.md
    tool.toml
    src/                 # also contains the Rust module tests
```

`tools/_template/` is an adaptable starter structure, not an included tool. Concrete tools are listed in the root catalog together with their README, implementation, and metadata. The `alias-manager/` example uses Rust's module-test layout and does not require a separate `tests/` directory.

## Adding a tool

1. Choose a unique, stable directory name and use the same ID in `tool.toml`.
2. Define the use case, users, minimum feature set, language, prerequisites, and target platforms.
3. Set a platform status for Windows, Linux, and macOS. Keep untested platforms `unverified`; mark unsupported platforms `unsupported`.
4. Document installation, usage, platform-specific steps, and limitations in the tool README. List a platform as supported in both the catalog and metadata only after suitable build and test evidence exists.
5. Add a catalog entry with the tool name, short description, and all three platform statuses.
