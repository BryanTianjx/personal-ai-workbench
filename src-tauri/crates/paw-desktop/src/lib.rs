//! paw-desktop: the desktop application boundary and the only composition
//! root. Tauri commands/events and dependency wiring belong here; the typed
//! command layer lives in `commands` and stays Tauri-free at its core
//! (`WorkManagementApi`) so the E1-05 UI-to-SQLite integration tests keep
//! driving the exact code path the UI drives.
//!
//! Dependency contract (ADR-018): the only crate allowed to depend on both
//! paw-application and paw-infrastructure. UI code must never access SQLite
//! or repositories directly.
//!
//! Tauri shell (E1 Tauri-shell unit): `run()` builds the native window,
//! resolves the authoritative local database (ADR-006: one `app.sqlite` in
//! the per-user app data directory), runs pending migrations and constructs
//! the command surface once at startup. Dependencies are constructed
//! explicitly here (E1-rust-workspace-layout §4: no service locator).

use std::fs;
use std::path::PathBuf;

use commands::WorkManagementApi;
use paw_infrastructure::sqlite;
use tauri::Manager;

pub mod commands;

/// Resolve the SQLite migrations directory.
///
/// Resolution order:
/// 1. `PAW_MIGRATIONS_DIR` environment variable (tests, packaging overrides);
/// 2. `migrations/sqlite` next to the running executable (deployed layout);
/// 3. the source-tree location relative to this crate's manifest (dev and
///    evidence builds run from the workspace checkout, so the absolute
///    compile-time path is valid on the build machine).
fn resolve_migrations_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PAW_MIGRATIONS_DIR") {
        return PathBuf::from(dir);
    }
    let deployed = std::env::current_exe().ok().and_then(|exe| {
        exe.parent()
            .map(|parent| parent.join("migrations").join("sqlite"))
    });
    if let Some(candidate) = deployed
        && candidate.is_dir()
    {
        return candidate;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

/// Bootstrap the persistent application state (ADR-006: one authoritative
/// `app.sqlite` in the per-user app data directory; migrations run before
/// the command surface opens).
fn bootstrap_state(app: &tauri::App) -> Result<WorkManagementApi, Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("app.sqlite");
    {
        let mut conn = sqlite::open(&db_path)?;
        sqlite::migrate(&mut conn, &resolve_migrations_dir())?;
        // Connection dropped here: the API reopens the database with the
        // connection policy; a single connection is held at a time (E1
        // desktop is single-window, single-user).
    }

    // The FTS projections are rebuildable and never authoritative: rebuild
    // once after migrations so upgrades re-index existing content (E2-03).
    {
        let search = paw_infrastructure::search::SqliteSearchService::open(&db_path)?;
        search.rebuild()?;
    }

    let api = WorkManagementApi::open(&db_path)?;
    Ok(api)
}

/// Run the native Tauri shell.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_workspaces,
            commands::create_workspace,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::list_projects,
            commands::create_area,
            commands::update_area,
            commands::delete_area,
            commands::list_areas,
            commands::create_goal,
            commands::update_goal,
            commands::delete_goal,
            commands::list_goals,
            commands::create_task,
            commands::update_task_details,
            commands::transition_task,
            commands::delete_task,
            commands::list_tasks,
            commands::search,
            commands::dashboard,
            commands::add_attachment,
            commands::list_attachments,
            commands::read_attachment,
            commands::read_attachment_bytes,
            commands::open_attachment_externally,
            commands::remove_attachment,
            commands::list_operations,
            commands::request_operation_cancellation,
            commands::resume_operation,
            commands::list_quarantined_messages,
            commands::redrive_message,
            commands::export_operations,
            commands::purge_expired_data,
            commands::list_roadmaps,
            commands::create_roadmap,
            commands::delete_roadmap,
            commands::list_plans,
            commands::create_plan,
            commands::update_plan_status,
            commands::list_items,
            commands::create_item,
            commands::transition_item,
            commands::link_item_to_project,
            commands::create_task_for_item,
            commands::append_session,
            commands::list_sessions,
            commands::list_knowledge_refs,
            commands::create_knowledge_ref,
            commands::set_knowledge_availability,
            commands::delete_knowledge_ref,
        ])
        .setup(|app| {
            let api = bootstrap_state(app)?;
            app.manage(std::sync::Mutex::new(api));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Tauri application");
}
