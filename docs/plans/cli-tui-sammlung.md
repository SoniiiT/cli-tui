# Implementation plan: CLI/TUI collection for Windows, Linux, and macOS

## Goal

This repository is intended to host multiple standalone CLI and TUI applications for different use cases. Each tool should clearly state its purpose, prerequisites, and supported operating systems. A tool may support Windows, Linux, macOS, or any subset of them; cross-platform support is decided per tool rather than assumed globally.

## Initial state and scope

- At the time this plan was written, the repository contained only a brief `README.md` describing the convention of one subdirectory per application/tool.
- There was no concrete CLI/TUI feature, programming language, build environment, or test infrastructure to guide an implementation.
- The initial scope was limited to repository conventions and a reusable tool scaffold. No specific tool or TUI was to be implemented in that phase.
- The first tool's concrete purpose, language, toolchain, and target platforms had to be selected before implementation. Build/test automation would then be added to match the selected toolchain.

## Proposed structure

```text
README.md                 # entry point and tool catalog
docs/
  platform-support.md     # platform status definitions and test requirements
  plans/                  # implementation plans
tools/
  README.md
  _template/               # template, not an included tool
    README.md
    tool.toml
    src/
    tests/
  <tool-id>/
    README.md             # purpose, installation, usage, limitations
    tool.toml             # metadata, including platform support
    src/                  # tool implementation
    tests/                # tool-specific tests
```

This structure is a convention, not a requirement for a particular programming language or build system. A shared build/test workflow should be introduced only if it fits the first tool's chosen toolchain.

## Implementation steps

1. **Define repository conventions**
   - Expand `README.md` to explain the repository purpose and the one-directory-per-tool rule.
   - Add a tool catalog with the name, short description, and platform status.
   - Define a simple metadata format for each tool. Minimum fields: stable tool ID, purpose, runtime/build prerequisites, and platform statuses for `windows`, `linux`, and `macos`.
   - Treat platform values as a set: for example, `[linux, macos]` means support for both systems, not Windows.

2. **Define the platform contract**
   - State in `docs/platform-support.md` that support is documented and tested per tool.
   - Distinguish supported, unsupported, and unverified; do not treat missing testing as support.
   - Document platform-dependent features and installation steps in each tool's own documentation. Keep shared logic platform-neutral where practical.

3. **Select the first tool and implement a vertical slice**
   - Before coding, choose the use case, users, minimum feature set, programming language, and target platforms.
   - Create the tool in `tools/<tool-id>/` according to the conventions and build/test it locally on at least one declared target platform.
   - Document help, installation/startup, failure cases, and known platform limitations.

4. **Add automated checks**
   - Choose formatting, static analysis, build, and test commands for the selected language.
   - Add CI jobs for the tool's declared platforms; undeclared platforms need not block CI.
   - Run platform-dependent tests natively on the relevant runner or document any limitation clearly. Cross-compilation alone is not runtime evidence.
   - Check consistency between the tool catalog and tool metadata.

5. **Reuse the pattern for future tools**
   - Add new tools independently without forcing them into a shared application or release cycle.
   - Extract shared libraries only when concrete reuse emerges and their dependencies are viable on all affected target platforms.

## Acceptance criteria

- The repository home page explains that the project is a collection of standalone CLI/TUI tools and that each tool has its own directory.
- Each included tool's purpose, prerequisites, and supported operating systems are discoverable.
- A tool can explicitly declare support for Linux only, macOS only, Windows only, any pair, or all three platforms.
- `supported` is backed by successful builds/tests appropriate to the declared platforms or by a clearly documented manual verification.
- Tool-specific documentation matches the catalog and metadata.

## Open decisions

- What concrete first CLI/TUI tool should be used as the example, and what problem will it solve?
- Which language and build/package management approach fit the first tool and the planned maintenance of the collection?
- Should CI use GitHub Actions or another platform, and which macOS/Windows/Linux runners are available?
- Which supported OS versions and processor architectures should be specified per platform?

## Risks and mitigations

- **Standardizing the toolchain too early:** may constrain tools unnecessarily. Standardize tool directories and metadata first; decide shared runtime or release requirements only after a concrete first tool exists.
- **Unclear platform claims:** “probably works” is not a reliable support status. Declare support only after installation and the core path have been verified on the target system.
- **Platform-specific terminal differences:** input, ANSI colors, signals, file paths, and terminal dimensions may vary. Cover these cases in acceptance tests and document differences.
- **Too much initial scope:** complete the catalog, platform conventions, and one representative tool first; prioritize later tools independently.

## Recommended sequence

Define repository conventions and the platform contract → select the first tool → build a vertical slice and test target systems → add CI → reuse the pattern for future tools.

## Implementation status at the time of writing

The repository scaffold, tool catalog, and platform contract had been prepared. `tools/_template/` provided a copyable structure and metadata template, but was not an included tool. At that stage, implementation of the first tool and related CI remained open pending decisions about the use case, language, and target platforms.
