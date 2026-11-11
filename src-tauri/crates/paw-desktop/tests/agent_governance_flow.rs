//! E5-06 end-to-end flow through the desktop command surface (the exact
//! code path the UI drives): register an agent, grant a capability, propose
//! a governed invocation, approve the high-risk path, and reconcile.
//! Authorization denial and scope isolation are asserted through direct
//! backend calls (Gate C discipline).

use std::path::PathBuf;

use paw_infrastructure::sqlite;

fn temp_db(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "paw-e5-e2e-{tag}-{}",
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
fn agent_governance_flow_end_to_end() {
    let db_path = temp_db("flow");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");

    // Workspace for the whole flow.
    let workspace = api.create_workspace("E5 e2e").expect("workspace");

    // 1. Register an agent (human-only action, gated + audited).
    let registration = api
        .register_agent(&workspace.id, "fake-worker", "fake")
        .expect("registered");
    assert_eq!(registration.display_name, "fake-worker");
    let agents = api.list_agents(&workspace.id).expect("listed");
    assert_eq!(agents.len(), 1);

    // 2. Before any grant: the invocation is denied by default (the
    // pipeline evaluates the agent subject's grants fresh).
    let denied = api.propose_agent_invocation(
        &workspace.id,
        registration.id.as_str(),
        "work_management.task.create",
        "1.0.0",
        "{}",
    );
    assert!(
        matches!(denied, Err(paw_desktop::commands::CommandError::Denied)),
        "default deny without a grant"
    );

    // 3. Grant the capability to the agent subject (human-only action).
    let grant_id = api
        .create_agent_grant(
            &workspace.id,
            &format!("agent:{}", registration.id.as_str()),
            vec!["work_management.task.create".to_string()],
            None,
        )
        .expect("grant created");

    // 4. Low-risk capability: runs to completion in one propose call.
    let outcome = api
        .propose_agent_invocation(
            &workspace.id,
            registration.id.as_str(),
            "work_management.task.create",
            "1.0.0",
            "{}",
        )
        .expect("completed");
    match outcome.status {
        paw_desktop::commands::AgentProposalStatus::Completed => {
            let execution = &outcome.execution;
            assert_eq!(
                execution.status,
                paw_application::ExecutionStatus::Completed
            );
            let operation = outcome.operation.as_ref().expect("envelope");
            assert_eq!(operation.status, "Succeeded");
            // Attempt trail: started → accepted → completed.
            let attempts = api.list_operations(&workspace.id).expect("queue");
            assert!(
                attempts.iter().any(|op| op.id == operation.id
                    && op.status == paw_infrastructure::operations::OperationStatus::Succeeded),
                "operation visible in the workspace queue"
            );
        }
        other => panic!("expected completed proposal, got {other:?}"),
    }

    // 5. High-risk capability: two-phase approval. Grant it first —
    // approval never substitutes for grant authorization (ADR-029 §4.3).
    api.create_agent_grant(
        &workspace.id,
        &format!("agent:{}", registration.id.as_str()),
        vec!["data.purge".to_string()],
        None,
    )
    .expect("high-risk grant created");
    let pending = api
        .propose_agent_invocation(
            &workspace.id,
            registration.id.as_str(),
            "data.purge",
            "1.0.0",
            "{}",
        )
        .expect("awaiting approval");
    let execution_id = match pending.status {
        paw_desktop::commands::AgentProposalStatus::AwaitingApproval => {
            let display = pending.display.as_ref().expect("approval display");
            assert_eq!(display.risk_class, "High");
            assert_eq!(display.capability_id, "data.purge");
            pending.execution.id.clone()
        }
        other => panic!("expected awaiting approval, got {other:?}"),
    };
    let approvals = api.list_pending_approvals(&workspace.id).expect("pending");
    assert_eq!(approvals.len(), 1, "the propose-time approval record");

    // Approve → the parked execution runs.
    let run = api
        .decide_approval(&workspace.id, execution_id.as_str(), true)
        .expect("approved and run")
        .expect("run outcome");
    assert!(matches!(
        run.status,
        paw_desktop::commands::AgentRunStatus::Completed
    ));
    assert_eq!(
        run.execution.status,
        paw_application::ExecutionStatus::Completed
    );

    // 6. Cross-workspace isolation: another workspace cannot see or decide
    // on this workspace's executions.
    let other = api.create_workspace("other").expect("other workspace");
    let cross = api.decide_approval(&other.id, execution_id.as_str(), true);
    assert!(
        matches!(
            cross,
            Err(paw_desktop::commands::CommandError::AgentExecutionNotFound)
        ),
        "cross-workspace executions are invisible"
    );
    let cross_ops = api
        .list_agent_executions(&other.id, 50)
        .expect("other workspace executions");
    assert!(cross_ops.is_empty());

    // 7. Revoke the grant: the next invocation is denied again (fresh
    // grant reads, ADR-029 §4.4).
    api.revoke_agent_grant(&workspace.id, &grant_id)
        .expect("revoked");
    let denied_after_revocation = api.propose_agent_invocation(
        &workspace.id,
        registration.id.as_str(),
        "work_management.task.create",
        "1.0.0",
        "{}",
    );
    assert!(matches!(
        denied_after_revocation,
        Err(paw_desktop::commands::CommandError::Denied)
    ));

    // Cleanup best-effort.
    let _ = std::fs::remove_dir_all(db_path.parent().expect("temp parent"));
}

#[test]
fn suspended_registration_blocks_invocation_and_grant_isolation() {
    let db_path = temp_db("suspended");
    let api = paw_desktop::commands::WorkManagementApi::open(&db_path).expect("api");
    let workspace = api.create_workspace("suspended test").expect("workspace");
    let registration = api
        .register_agent(&workspace.id, "paused-agent", "fake")
        .expect("registered");

    // Suspend (human action; state machine Active → Suspended).
    let suspended = api
        .update_agent_registration(&workspace.id, registration.id.as_str(), "Suspended")
        .expect("suspended");
    assert_eq!(
        suspended.status,
        paw_application::RegistrationStatus::Suspended
    );

    // Grant present, but the registration is not active: proposal refuses.
    api.create_agent_grant(
        &workspace.id,
        &format!("agent:{}", registration.id.as_str()),
        vec!["work_management.task.create".to_string()],
        None,
    )
    .expect("grant created");
    let outcome = api.propose_agent_invocation(
        &workspace.id,
        registration.id.as_str(),
        "work_management.task.create",
        "1.0.0",
        "{}",
    );
    assert!(matches!(
        outcome,
        Err(paw_desktop::commands::CommandError::AgentRegistrationNotFound)
    ));

    // Retired is terminal: no transition back to Active.
    api.update_agent_registration(&workspace.id, registration.id.as_str(), "Retired")
        .expect("retired");
    let reactivate =
        api.update_agent_registration(&workspace.id, registration.id.as_str(), "Active");
    assert!(reactivate.is_err(), "Retired is terminal");

    let _ = std::fs::remove_dir_all(db_path.parent().expect("temp parent"));
}
