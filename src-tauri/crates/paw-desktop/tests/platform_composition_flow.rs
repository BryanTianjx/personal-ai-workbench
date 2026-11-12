//! E7 end-to-end flow through the desktop command surface: static plugin
//! catalog with dependency DAG, per-workspace disable preference gating NEW
//! invocations while data/history stay intact, role profiles as
//! experience-only templates, and workspace compositions scoped to one
//! Data Workspace. Composition/profile never grant authority — the
//! authorization layer is untouched.

use std::path::PathBuf;

use paw_infrastructure::sqlite;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "paw-e7-e2e-{tag}-{}",
        uuid::Uuid::now_v7().simple()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let migrations = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite");
    let db_path = dir.join("app.sqlite");
    let mut conn = sqlite::open(&db_path).expect("open db");
    sqlite::migrate(&mut conn, &migrations).expect("migrate");
    db_path
}

#[test]
fn platform_composition_flow_end_to_end() {
    let db_path = temp_db("flow");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");
    let workspace = api.create_workspace("E7 e2e").expect("workspace");

    // 1. Static plugin catalog: all first-party plugins available.
    let states = api.list_plugin_states(&workspace.id).expect("states");
    assert_eq!(states.len(), 5, "five first-party plugins registered");
    assert!(states.iter().all(|(_, disabled)| !disabled));

    // 2. Disable core.ai_collaboration → its capabilities refuse NEW
    //    invocations (data/history untouched — the disable is a preference).
    api.set_plugin_disabled(&workspace.id, "core.ai_collaboration", true)
        .expect("disabled");
    let states = api.list_plugin_states(&workspace.id).expect("states");
    let ai = states
        .iter()
        .find(|(id, _)| id == "core.ai_collaboration")
        .expect("ai plugin listed");
    assert!(ai.1, "disabled preference persisted");

    // 3. Role profile: experience template, no authority.
    let profile = api
        .create_role_profile(&workspace.id, "工程师视角", Vec::new(), None, Vec::new())
        .expect("profile created");
    let profiles = api.list_role_profiles(&workspace.id).expect("profiles");
    assert_eq!(profiles.len(), 1);

    // 4. Composition scoped to actor + workspace.
    let composition = api
        .create_workspace_composition(
            &workspace.id,
            "工程师组合",
            Some(profile.id.as_str()),
            Vec::new(),
        )
        .expect("composition created");
    let compositions = api
        .list_workspace_compositions(&workspace.id)
        .expect("compositions");
    assert_eq!(compositions.len(), 1);
    assert_eq!(compositions[0].actor_id, "local-user");

    // 5. Cross-workspace isolation: another workspace sees nothing and
    //    cannot inherit the disable preference.
    let other = api.create_workspace("other").expect("other");
    let other_states = api.list_plugin_states(&other.id).expect("other states");
    assert!(other_states.iter().all(|(_, disabled)| !disabled));
    let other_profiles = api.list_role_profiles(&other.id).expect("empty");
    assert!(other_profiles.is_empty());
    let _ = composition;

    // 6. Re-enable → preference cleared.
    api.set_plugin_disabled(&workspace.id, "core.ai_collaboration", false)
        .expect("re-enabled");
    let states = api.list_plugin_states(&workspace.id).expect("states");
    let ai = states
        .iter()
        .find(|(id, _)| id == "core.ai_collaboration")
        .expect("ai plugin");
    assert!(!ai.1, "re-enabled");

    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}

#[test]
fn disabled_plugin_blocks_new_invocation() {
    let db_path = temp_db("gate");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");
    let workspace = api.create_workspace("gate test").expect("workspace");

    // Agent + grant for a capability owned by core.work_management.
    let registration = api
        .register_agent(&workspace.id, "gated-agent", "fake")
        .expect("registered");
    api.create_agent_grant(
        &workspace.id,
        &format!("agent:{}", registration.id.as_str()),
        vec!["work_management.task.create".to_string()],
        None,
    )
    .expect("grant created");

    // Baseline: invocation works while the plugin is available.
    let outcome = api.propose_agent_invocation(
        &workspace.id,
        registration.id.as_str(),
        "work_management.task.create",
        "1.0.0",
        "{}",
    );
    assert!(matches!(
        outcome,
        Ok(paw_desktop::commands::AgentProposalOutcome {
            status: paw_desktop::commands::AgentProposalStatus::Completed,
            ..
        })
    ));

    // Disable the owning plugin → NEW invocations refused (typed error),
    // while the agent grant itself remains valid data.
    api.set_plugin_disabled(&workspace.id, "core.work_management", true)
        .expect("disabled");
    let blocked = api.propose_agent_invocation(
        &workspace.id,
        registration.id.as_str(),
        "work_management.task.create",
        "1.0.0",
        "{}",
    );
    assert!(
        matches!(
            blocked,
            Err(paw_desktop::commands::CommandError::PluginDisabled)
        ),
        "disabled plugin blocks new invocations"
    );

    // Existing history is untouched by the disable (ADR-019: drain/reconcile,
    // never deletion) — the prior invocation's execution record survives.
    let executions = api
        .list_agent_executions(&workspace.id, 50)
        .expect("executions");
    assert!(!executions.is_empty(), "prior execution record survives");
    assert!(
        executions
            .iter()
            .all(|e| e.status == paw_application::ExecutionStatus::Completed),
        "completed executions are not altered by a later disable"
    );

    let _ = std::fs::remove_dir_all(db_path.parent().expect("parent"));
}
