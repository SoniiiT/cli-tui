# Platform support

Platform support is assessed separately for each tool. A tool's support status says nothing about other tools. A tool may support Windows, Linux, macOS, any single platform, any pair, or all three.

## Status values

Every `tool.toml` must list Windows, Linux, and macOS individually and assign one of these statuses to each:

| Status | Meaning | Catalog label |
| --- | --- | --- |
| `supported` | Installation and the documented core path have been successfully tested on the platform. | Supported |
| `unsupported` | The tool does not support the platform. | Unsupported |
| `unverified` | Support has not been demonstrated or the platform has not been tested. | Unverified |

Missing metadata is not evidence of support. Do not mark a platform `supported` merely because the tool might theoretically run there.

## Metadata format

Each tool has a `tool.toml` containing a stable ID, name, purpose, runtime and build prerequisites, and exactly one status for each platform. Use an empty list when there are no additional prerequisites.

```toml
id = "example-tool"
name = "Example Tool"
purpose = "A short description of the use case."
runtime = ["Runtime or minimum version"]
build = ["Compiler or build tool"]

[platforms.windows]
status = "unverified"
evidence = ""

[platforms.linux]
status = "unverified"
evidence = ""

[platforms.macos]
status = "unverified"
evidence = ""
```

This example is a template, not an included or tested tool. Set `status = "unsupported"` for unsupported platforms. Evidence for `supported` should briefly state the test/build run, OS version, architecture, and test date. For `unsupported` and `unverified`, leave `evidence` empty or describe a known limitation or outstanding test as appropriate.

## Evidence for `supported`

Evidence must cover installation and the user-relevant core path on the actual target operating system:

- Successfully build and test the tool on the declared platform, preferably on a native system or CI runner.
- Record the OS version and architecture alongside the result.
- Check relevant terminal differences, especially input, ANSI colors, signals, file paths, and terminal dimensions.
- Document installation instructions and known limitations in the tool README.

Cross-compilation alone does not demonstrate runtime support. If a native run is unavailable, document the limitation and keep the status `unverified` until sufficient evidence is available.

## Catalog maintenance

The tool catalog in the root README must reflect each tool's name, short description, and Windows/Linux/macOS statuses from its `tool.toml`. Keep both locations in sync when statuses or test results change. OS versions and processor architectures are currently recorded in the evidence; each tool defines its own specific target versions.
