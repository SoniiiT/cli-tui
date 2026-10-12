//! Core logic for venv-runner: project and venv detection, Python interpreter
//! discovery, and the command plans that create a venv or run a project in it.
//! The TUI in `main.rs` only drives these pieces.

pub mod plan;
pub mod project;
pub mod python;
