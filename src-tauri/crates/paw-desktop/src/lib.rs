//! paw-desktop: the desktop application boundary and the only composition
//! root. Tauri commands/events and dependency wiring belong here; the typed
//! command layer lives in `commands` and is Tauri-free until the Tauri shell
//! lands so it stays integration-testable (E1-05).
//!
//! Dependency contract (ADR-018): the only crate allowed to depend on both
//! paw-application and paw-infrastructure. UI code must never access SQLite
//! or repositories directly.

pub mod commands;
