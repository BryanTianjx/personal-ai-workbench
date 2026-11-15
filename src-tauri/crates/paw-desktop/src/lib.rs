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
/// 2. `migrations/sqlite` next to the running executable — the deployed
///    layout. `bundle.resources` maps `src-tauri/migrations/sqlite/*.sql`
///    there for NSIS/MSI installs (on Windows the resource directory is the
///    executable directory), so a clean install finds migrations without a
///    source checkout (PAW-AUDIT-001);
/// 3. the source-tree location relative to this crate's manifest — only
///    accepted when it actually exists at runtime (dev and evidence builds
///    run from the workspace checkout).
///
/// If none of these exist the error is explicit and actionable instead of a
/// cryptic `read_dir` failure inside `sqlite::migrate`.
fn resolve_migrations_dir() -> Result<PathBuf, String> {
    let deployed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|parent| parent.join("migrations").join("sqlite")));
    pick_migrations_dir(
        std::env::var_os("PAW_MIGRATIONS_DIR"),
        deployed,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite"),
    )
}

/// Pure decision core of [`resolve_migrations_dir`], separated so the
/// precedence rules are testable without mutating process state.
fn pick_migrations_dir(
    env_override: Option<std::ffi::OsString>,
    deployed: Option<PathBuf>,
    source: PathBuf,
) -> Result<PathBuf, String> {
    if let Some(dir) = env_override {
        return Ok(PathBuf::from(dir));
    }
    if let Some(candidate) = deployed.filter(|candidate| candidate.is_dir()) {
        return Ok(candidate);
    }
    if source.is_dir() {
        return Ok(source);
    }
    Err(String::from(
        "SQLite migrations directory not found: expected migrations/sqlite next \
         to the executable (bundled via bundle.resources) or set PAW_MIGRATIONS_DIR",
    ))
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
        sqlite::migrate(&mut conn, &resolve_migrations_dir()?)?;
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
            commands::register_agent,
            commands::update_agent_registration,
            commands::list_agents,
            commands::propose_agent_invocation,
            commands::decide_approval,
            commands::revoke_approval,
            commands::reconcile_agent_execution,
            commands::list_agent_executions,
            commands::list_pending_approvals,
            commands::create_agent_grant,
            commands::revoke_agent_grant,
            commands::list_agent_grants,
            commands::create_skill,
            commands::update_skill,
            commands::list_skills,
            commands::add_skill_evidence,
            commands::list_skill_evidence,
            commands::create_assessment,
            commands::list_assessments,
            commands::propose_learning_gap,
            commands::confirm_learning_gap,
            commands::dismiss_learning_gap,
            commands::list_learning_gaps,
            commands::create_creator_topic,
            commands::update_creator_topic,
            commands::list_creator_topics,
            commands::create_creator_draft,
            commands::append_creator_draft_version,
            commands::transition_creator_draft,
            commands::submit_creator_review,
            commands::list_creator_drafts,
            commands::list_creator_reviews,
            commands::record_publish,
            commands::list_publish_records,
            commands::record_retrospective,
            commands::list_retrospectives,
            commands::list_item_project_links,
            commands::set_plugin_disabled,
            commands::list_plugin_states,
            commands::create_role_profile,
            commands::list_role_profiles,
            commands::create_workspace_composition,
            commands::list_workspace_compositions,
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

#[cfg(test)]
mod migration_resolution_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// An existing directory to stand in for candidates that must "exist".
    fn existing_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("paw-mig-resolve-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn nonexistent_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("paw-mig-missing-{tag}-{}", std::process::id()));
        assert!(!dir.exists(), "test path unexpectedly exists: {dir:?}");
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn explicit_env_override_wins_over_everything() {
        let env_dir = existing_dir("env-override");
        let deployed = existing_dir("override-deployed");
        let source = existing_dir("override-source");
        let resolved = pick_migrations_dir(
            Some(env_dir.clone().into_os_string()),
            Some(deployed.clone()),
            source.clone(),
        )
        .unwrap();
        assert_eq!(resolved, env_dir);
        cleanup(&env_dir);
        cleanup(&deployed);
        cleanup(&source);
    }

    #[test]
    fn deployed_layout_wins_over_source_tree() {
        let deployed = existing_dir("wins-deployed");
        let source = existing_dir("wins-source");
        let resolved = pick_migrations_dir(None, Some(deployed.clone()), source.clone()).unwrap();
        assert_eq!(resolved, deployed);
        cleanup(&deployed);
        cleanup(&source);
    }

    #[test]
    fn deployed_candidate_must_exist_to_qualify() {
        let missing_deployed = nonexistent_dir("qualify-deployed");
        let source = existing_dir("qualify-source");
        let resolved =
            pick_migrations_dir(None, Some(missing_deployed.clone()), source.clone()).unwrap();
        assert_eq!(resolved, source);
        cleanup(&source);
    }

    #[test]
    fn source_tree_fallback_only_when_it_exists() {
        let missing_source = nonexistent_dir("fallback-source");
        let resolved = pick_migrations_dir(None, None, missing_source.clone());
        assert!(resolved.is_err(), "no candidate exists, resolution must fail");
        let message = resolved.unwrap_err();
        assert!(message.contains("PAW_MIGRATIONS_DIR"), "actionable message, got: {message}");
    }

    #[test]
    fn missing_deployed_with_existing_source_resolves_to_source() {
        let missing_deployed = nonexistent_dir("missing-deployed");
        let source = existing_dir("missing-source");
        let resolved =
            pick_migrations_dir(None, Some(missing_deployed.clone()), source.clone()).unwrap();
        assert_eq!(resolved, source);
        cleanup(&source);
    }

    #[test]
    fn real_source_tree_resolves_in_workspace_checkout() {
        // In a dev/evidence run from the checkout the source fallback must hold,
        // which is what every existing migration test relies on.
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite");
        let resolved = pick_migrations_dir(None, None, source.clone()).unwrap();
        assert_eq!(resolved, source);
    }
}
