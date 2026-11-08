//! paw-desktop: the desktop application boundary and the only composition
//! root. Tauri commands/events and dependency wiring belong here once the
//! Tauri shell lands (E1-05); until then this crate remains the placeholder
//! that preserves the workspace dependency contract.
//!
//! Dependency contract (ADR-018): the only crate allowed to depend on both
//! paw-application and paw-infrastructure. UI code must never access SQLite
//! or repositories directly.
