# Tools

Each application gets its own directory under `tools/`. These directories are independent projects and may use different languages, build systems, and release cycles.

## Creating a tool

1. Copy `tools/_template/` into a directory named with a stable tool ID.
2. Document the purpose, prerequisites, installation, usage, failure cases, and platform limitations in `README.md`.
3. Add the ID, name, purpose, runtime/build prerequisites, and one status for each of Windows, Linux, and macOS to `tool.toml`.
4. Mark platforms `supported` and add the tool to the root catalog (`README.md`) only after suitable build and test evidence is available.

Status definitions and evidence requirements are in [`../docs/platform-support.md`](../docs/platform-support.md).

`_template/` is a template, not an application, so it is not listed in the tool catalog.
