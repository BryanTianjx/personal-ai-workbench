//! Backup & restore integration test (PAW-AUDIT-002 remediation evidence).
//!
//! Drives the desktop backup layer exactly as the app will: snapshot via the
//! managed backups directory, stage a restore, apply it at "bootstrap" (the
//! same `apply_pending_restore` call the shell makes before any connection
//! opens), and verify the authoritative data and the managed attachment
//! directory both end up in the post-restore state.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use paw_desktop::backup::{
    PendingRestoreOutcome, apply_pending_restore, create_backup_into, list_managed_backups,
    stage_restore,
};
use paw_desktop::commands::WorkManagementApi;
use paw_infrastructure::sqlite;

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_dir(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("paw-backup-flow-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

fn migrated_db(path: &Path) {
    let mut conn = sqlite::open(path).unwrap();
    sqlite::migrate(&mut conn, &migrations_dir()).unwrap();
}

#[test]
fn snapshot_stage_and_bootstrap_restore_round_trip() {
    let data_dir = unique_dir("roundtrip");
    let db_path = data_dir.join("app.sqlite");
    migrated_db(&db_path);

    // 1. Snapshot the database in state A (one workspace marker).
    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        api.create_workspace("snapshot-marker").unwrap();
    }
    let snapshot = create_backup_into(&db_path, &data_dir).unwrap();
    assert_eq!(snapshot.info.attachments_excluded, true);
    assert!(snapshot.size_bytes > 0);
    assert!(snapshot.file_name.starts_with("backup-"));

    // The managed listing shows the snapshot as valid.
    let listed = list_managed_backups(&data_dir);
    assert_eq!(listed.len(), 1);
    assert!(listed[0].valid);
    assert_eq!(listed[0].file_name, snapshot.file_name);

    // 2. Mutate the database past the snapshot (second workspace).
    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        api.create_workspace("after-snapshot").unwrap();
        let names: Vec<String> = api
            .list_workspaces()
            .unwrap()
            .into_iter()
            .map(|w| w.name)
            .collect();
        assert_eq!(names.len(), 2);
    }

    // 3. Stage the restore: validated copy next to the active database.
    let backup_path = data_dir.join("backups").join(&snapshot.file_name);
    stage_restore(&db_path, &backup_path).unwrap();
    assert!(db_path.with_extension("restore-pending").is_file());

    // 4. Bootstrap applies the staged restore before any connection opens.
    let outcome = apply_pending_restore(&db_path).unwrap();
    assert_eq!(outcome, PendingRestoreOutcome::Restored);
    assert!(!db_path.with_extension("restore-pending").exists());

    // 5. The authoritative data is the snapshot's, and the staged file is
    //    gone. Restore is idempotent on migration state: the snapshot is on
    //    the same schema version, so reopening needs no further migrations.
    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        let names: Vec<String> = api
            .list_workspaces()
            .unwrap()
            .into_iter()
            .map(|w| w.name)
            .collect();
        assert_eq!(names, vec!["snapshot-marker".to_string()]);
    }

    fs::remove_dir_all(&data_dir).unwrap();
}

#[test]
fn restore_clears_managed_attachments_of_the_restored_profile() {
    let data_dir = unique_dir("attachments");
    let db_path = data_dir.join("app.sqlite");
    migrated_db(&db_path);

    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        api.create_workspace("attachment-marker").unwrap();
    }
    let snapshot = create_backup_into(&db_path, &data_dir).unwrap();

    // Simulate attachment bytes that exist after the snapshot was taken:
    // REL-006 keeps bytes out of snapshots, so they are stale by definition.
    let attachments_root = data_dir.join("attachments");
    let stale = attachments_root.join("stale-workspace");
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("managed-file"), b"stale-bytes").unwrap();

    let backup_path = data_dir.join("backups").join(&snapshot.file_name);
    stage_restore(&db_path, &backup_path).unwrap();
    let outcome = apply_pending_restore(&db_path).unwrap();
    assert_eq!(outcome, PendingRestoreOutcome::Restored);

    // The attachment root is kept (empty): metadata and bytes cannot disagree.
    assert!(attachments_root.is_dir());
    assert_eq!(fs::read_dir(&attachments_root).unwrap().count(), 0);

    fs::remove_dir_all(&data_dir).unwrap();
}

#[test]
fn rejected_snapshot_leaves_active_database_untouched() {
    let data_dir = unique_dir("rejected");
    let db_path = data_dir.join("app.sqlite");
    migrated_db(&db_path);

    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        api.create_workspace("live-data").unwrap();
    }

    // Stage a garbage file (no paw_backup_metadata table): a foreign or
    // corrupt "snapshot".
    let pending = db_path.with_extension("restore-pending");
    fs::write(&pending, b"not a sqlite snapshot at all").unwrap();

    let outcome = apply_pending_restore(&db_path).unwrap();
    assert_eq!(outcome, PendingRestoreOutcome::Rejected);

    // The staged copy is removed, the live database still holds its data and
    // opens cleanly.
    assert!(!pending.exists());
    {
        let api = WorkManagementApi::open(&db_path).unwrap();
        let names: Vec<String> = api
            .list_workspaces()
            .unwrap()
            .into_iter()
            .map(|w| w.name)
            .collect();
        assert_eq!(names, vec!["live-data".to_string()]);
    }

    fs::remove_dir_all(&data_dir).unwrap();
}

#[test]
fn bootstrap_without_staged_restore_is_a_no_op() {
    let data_dir = unique_dir("nopending");
    let db_path = data_dir.join("app.sqlite");
    migrated_db(&db_path);

    let outcome = apply_pending_restore(&db_path).unwrap();
    assert_eq!(outcome, PendingRestoreOutcome::NoPending);

    fs::remove_dir_all(&data_dir).unwrap();
}
