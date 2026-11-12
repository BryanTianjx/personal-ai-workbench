use rusqlite::Connection;

#[test]
fn probe_user_db_readonly() {
    let db = std::env::var("PAW_PROBE_DB").expect("PAW_PROBE_DB");
    let conn = Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");

    println!("=== workspaces ===");
    for row in conn
        .prepare("SELECT name FROM workspaces ORDER BY created_at")
        .expect("q")
        .query_map([], |r| r.get::<_, String>(0))
        .expect("q")
    {
        println!("  {}", row.expect("row"));
    }

    println!("=== operations (kind, status) count by kind ===");
    let mut stmt = conn
        .prepare(
            "SELECT kind, status, COUNT(*) FROM operations GROUP BY kind, status ORDER BY kind, status",
        )
        .expect("q");
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })
    .expect("q")
    {
        let (k, s, c) = row.expect("row");
        println!("  {k} [{s}] x{c}");
    }

    println!("=== latest agent.invocation operations (12) ===");
    let mut stmt = conn
        .prepare(
            "SELECT o.kind, o.status, o.created_at, a.outcome, a.error_category
             FROM operations o LEFT JOIN operation_attempts a ON a.operation_id = o.id
             WHERE o.kind = 'agent.invocation'
             ORDER BY o.created_at DESC LIMIT 12",
        )
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .expect("q")
    {
        let (k, s, _t, outcome, err) = row.expect("row");
        println!("  {k} [{s}] attempt_outcome={outcome:?} err={err:?}");
    }

    println!("=== agent_executions (capability, status) ===");
    let mut stmt = conn
        .prepare(
            "SELECT capability_id, status, COUNT(*) FROM agent_executions GROUP BY capability_id, status",
        )
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .expect("q")
    {
        let (c, s, n) = row.expect("row");
        println!("  {c} [{s}] x{n}");
    }

    println!("=== outbox events by type ===");
    let mut stmt = conn
        .prepare("SELECT event_type, COUNT(*) FROM outbox_messages GROUP BY event_type ORDER BY event_type")
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .expect("q")
    {
        let (t, n) = row.expect("row");
        println!("  {t} x{n}");
    }

    println!("=== audit denied (last 10) ===");
    let mut stmt = conn
        .prepare(
            "SELECT action, resource_type, outcome, created_at FROM audit_events
             WHERE outcome = 'denied' ORDER BY created_at DESC LIMIT 10",
        )
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .expect("q")
    {
        let (a, rt, t) = row.expect("row");
        println!("  DENIED {a} ({rt}) at {t}");
    }

    println!("=== audit counts by action (E5/E6/E7 relevant) ===");
    let mut stmt = conn
        .prepare(
            "SELECT action, outcome, COUNT(*) FROM audit_events
             WHERE action LIKE 'agent.%' OR action LIKE 'approval.%' OR action LIKE 'skill.%'
                OR action LIKE 'learning.%' OR action LIKE 'creator.%' OR action LIKE 'plugin.%'
             GROUP BY action, outcome ORDER BY action",
        )
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .expect("q")
    {
        let (a, o, n) = row.expect("row");
        println!("  {a} [{o}] x{n}");
    }

    println!("=== approvals ===");
    let mut stmt = conn
        .prepare("SELECT capability_id, status, consumed_attempts, attempts_allowed FROM approvals")
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .expect("q")
    {
        let (c, s, used, allowed) = row.expect("row");
        println!("  {c} [{s}] {used}/{allowed}");
    }

    println!("=== creator drafts (title, status, version) ===");
    let mut stmt = conn
        .prepare("SELECT title, status, current_version_no FROM creator_drafts")
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .expect("q")
    {
        let (t, s, v) = row.expect("row");
        println!("  {t} [{s}] v{v}");
    }
    println!("=== publish records ===");
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM creator_publish_records")
        .expect("q");
    let n: i64 = stmt.query_row([], |r| r.get(0)).expect("q");
    println!("  count={n}");
    println!("=== retrospectives ===");
    let mut stmt = conn
        .prepare(
            "SELECT r.summary, COUNT(s.session_id) FROM project_retrospectives r
             LEFT JOIN retrospective_session_refs s ON s.retrospective_id = r.id
             GROUP BY r.id",
        )
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .expect("q")
    {
        let (s, n) = row.expect("row");
        println!("  \"{s}\" sessions={n}");
    }
    println!("=== gap proposals (status) ===");
    let mut stmt = conn
        .prepare("SELECT status, COUNT(*) FROM learning_gap_proposals GROUP BY status")
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .expect("q")
    {
        let (s, n) = row.expect("row");
        println!("  {s} x{n}");
    }
    println!("=== plugin states ===");
    let mut stmt = conn
        .prepare("SELECT plugin_id, disabled FROM plugin_states")
        .expect("q");
    for row in stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .expect("q")
    {
        let (p, d) = row.expect("row");
        println!("  {p} disabled={d}");
    }
    println!("=== skills / evidence / assessments ===");
    for (label, sql) in [
        ("skills", "SELECT COUNT(*) FROM skills"),
        ("evidence", "SELECT COUNT(*) FROM skill_evidence"),
        ("assessments", "SELECT COUNT(*) FROM learning_assessments"),
    ] {
        let n: i64 = conn
            .query_row(sql, [], |r| r.get(0))
            .expect("q");
        println!("  {label}={n}");
    }
}
