//! Desktop binary entry point: runs the Tauri shell owned by the
//! `paw-desktop` library crate (composition root, ADR-018).
//!
//! In release builds the process runs without a console window
//! (`windows_subsystem = "windows"`); debug builds keep a console so
//! diagnostics are visible during development and evidence runs.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    paw_desktop::run();
}
