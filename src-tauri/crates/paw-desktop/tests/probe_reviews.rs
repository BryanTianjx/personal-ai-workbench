use rusqlite::Connection;
#[test]
fn probe_reviews() {
    let db = std::env::var("PAW_PROBE_DB").expect("PAW_PROBE_DB");
    let conn = Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).expect("open");
    let mut stmt = conn.prepare("SELECT draft_id, version_no, decision, comment FROM creator_draft_reviews ORDER BY created_at").expect("q");
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?))).expect("q") {
        let (d, v, dec, c) = row.expect("row");
        println!("  review draft={d} v{v} {dec} comment={c:?}");
    }
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM approvals WHERE status = 'Consumed'", [], |r| r.get(0)).expect("q");
    println!("  approvals Consumed={n}");
}
