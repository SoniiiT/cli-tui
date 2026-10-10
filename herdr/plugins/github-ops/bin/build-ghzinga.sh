#!/usr/bin/env bash
set -euo pipefail

plugin_root="${HERDR_PLUGIN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
mkdir -p "$plugin_root/.runtime"
cargo install ghzinga --locked --version 0.5.1 --root "$plugin_root/.runtime" --force
