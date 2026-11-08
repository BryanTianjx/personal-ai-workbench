//! SQLite foundation for E1: connection policy, migration runner and schema
//! ledger. Follows the contract in
//! `docs/05-architecture-evolution/E1-sqlite-schema-migrations-workspace-isolation.md`
//! and ADR-006 (one authoritative local `app.sqlite`, workspace isolation by
//! trusted application context rather than UI filters).

use rusqlite::{Connection, params};
use sha2::Digest;
use std::fs;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io: {source}")]
    Io { source: std::io::Error },
    #[error("clock: {source}")]
    NowMsFailed { source: std::time::SystemTimeError },
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("checksum mismatch for migration {version}: expected {expected}, found {found}")]
    ChecksumMismatch {
        version: i64,
        expected: String,
        found: String,
    },
}

/// Open a database connection with the E1 connection policy.
///
/// Every pooled connection must have foreign keys enabled, WAL for desktop
/// concurrency and a bounded busy timeout. `synchronous` is set to FULL so
/// that a crash cannot truncate a committed transaction; this default can be
/// revisited only with measured evidence in `docs/02-architecture/data-architecture.md`.
pub fn open(path: &std::path::Path) -> Result<Connection, Error> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", 1)?;
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "busy_timeout", 15 * 60 * 1000)?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(conn)
}

/// Run pending migrations in ascending order against `conn`.
///
/// Migration files are `NNNN_name.sql` under `migrations_dir`. Each file is
/// applied inside one transaction and its SHA-256 checksum is recorded in the
/// `schema_migrations` ledger in the same transaction. A checksum mismatch for
/// an already-applied migration is a hard integrity error; the database is not
/// silently continued. See ADR-006 and
/// `E1-sqlite-schema-migrations-workspace-isolation.md` §2.
pub fn migrate(conn: &mut Connection, migrations_dir: &std::path::Path) -> Result<(), Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version         INTEGER PRIMARY KEY,
            name            TEXT NOT NULL,
            checksum        TEXT NOT NULL,
            applied_at      INTEGER NOT NULL
        )",
    )?;

    let mut files: Vec<std::path::PathBuf> = fs::read_dir(migrations_dir)
        .map_err(|e| Error::Io { source: e })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|p| p.extension().map(|e| e == "sql").unwrap_or(false))
        .collect();
    files.sort_by_key(|p| {
        let digits: String = p
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse::<i64>().unwrap_or(i64::MAX)
    });

    let applied: std::collections::HashMap<i64, (String, String, i64)> = {
        let mut stmt =
            conn.prepare("SELECT version, name, checksum, applied_at FROM schema_migrations")?;
        let mut rows = stmt.query([])?;
        let mut map = std::collections::HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            map.insert(
                row.get::<_, i64>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ),
            );
        }
        map
    };

    let tx = conn.unchecked_transaction()?;

    for path in &files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
        let version: i64 = digits.parse().unwrap_or(i64::MAX);

        let sql = fs::read_to_string(path).map_err(|e| Error::Io { source: e })?;
        let checksum = sha256_hex(sql.as_bytes());

        // Skip an already-applied migration only after verifying its checksum.
        if applied.contains_key(&version) {
            if let Some(stored_checksum) = conflicting_checksum(&applied, version, &checksum) {
                return Err(Error::ChecksumMismatch {
                    version,
                    expected: stored_checksum,
                    found: checksum,
                });
            }
            continue;
        }

        // A checksum that does not match the ledger row is a hard failure.

        tx.execute_batch(&sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![version, name, checksum, now_ms()?],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Deterministic UTC epoch-milliseconds timestamp (used for migration
/// `applied_at` and row timestamps). Marked `pub(crate)` so the SQLite
/// repository adapter (workspaces/projects) can stamp rows.
pub(crate) fn now_ms() -> Result<i64, Error> {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::NowMsFailed { source: e })?;
    Ok(d.as_millis() as i64)
}

/// Migrations directory relative to this crate, for in-crate tests:
/// `<workspace-root>/migrations/sqlite`. Integration tests under
/// `crates/*/tests/` resolve the same directory from their own manifest
/// directory instead.
#[cfg(test)]
pub(crate) fn migrations_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
}

/// Deterministic hex SHA-256 of `data`, without extra dependencies.
fn sha256_hex(data: &[u8]) -> String {
    let digest = sha2::Sha256::digest(data);
    let mut out = [0u8; 64];
    for (i, byte) in digest.iter().enumerate() {
        let hi = (*byte >> 4) & 0x0f;
        let lo = *byte & 0x0f;
        out[i * 2] = match hi {
            0..=9 => b'0' + hi,
            _ => b'a' + (hi - 10),
        };
        out[i * 2 + 1] = match lo {
            0..=9 => b'0' + lo,
            _ => b'a' + (lo - 10),
        };
    }
    String::from_utf8(out.to_vec()).unwrap()
}

/// If `version` was already applied with a *different* checksum, returns that
/// stored checksum (a conflict). Otherwise `None`.
fn conflicting_checksum(
    applied: &std::collections::HashMap<i64, (String, String, i64)>,
    version: i64,
    checksum: &str,
) -> Option<String> {
    applied
        .get(&version)
        .and_then(|(_stored_name, stored_checksum, _applied_at)| {
            if *stored_checksum != checksum {
                Some(stored_checksum.clone())
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/tmp-sqlite-test-{:?}-{:?}",
            std::thread::current().id(),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("test.db")
    }

    #[test]
    fn open_enables_foreign_keys_wal_and_busy_timeout() {
        let db = temp_db();
        let conn = open(&db).unwrap();
        let fk: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, i64>(0))
            .unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap();
        assert_eq!(fk, 1);
        assert_eq!(mode, "wal");
        // Close the connection (and its WAL/SHM sidecar files) before
        // cleanup: Windows locks files held open by this process.
        drop(conn);
        fs::remove_dir_all(db.parent().unwrap()).unwrap();
    }

    #[test]
    fn empty_database_initializes_and_records_checksum() {
        let db = temp_db();
        let mut conn = open(&db).unwrap();
        let migrations_dir = migrations_dir();
        migrate(&mut conn, &migrations_dir).unwrap();

        let ledger: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ledger, 1);

        let tables: String = conn
            .query_row(
                "SELECT group_concat(name) FROM sqlite_master WHERE type = 'table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(tables.contains("schema_migrations"));
        assert!(tables.contains("workspaces"));
        assert!(tables.contains("projects"));
        drop(conn);
        fs::remove_dir_all(db.parent().unwrap()).unwrap();
    }

    #[test]
    fn applied_checksum_mismatch_fails_closed() {
        let db = temp_db();
        let mut conn = open(&db).unwrap();

        // Establish a clean schema with the migration 0001 ledger row.
        migrate(&mut conn, &migrations_dir()).unwrap();

        // Seed a ledger row whose checksum disagrees with the migration file.
        let now = now_ms().unwrap();
        conn.execute(
            "INSERT INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (2, '0002_fake.sql', 'deadbeef', ?1)",
            [now],
        )
        .unwrap();

        // A migration file whose checksum differs from the ledger must fail
        // closed, and must not create any table.
        let bad_dir = temp_dir();
        fs::write(bad_dir.join("0002_fake.sql"), "SELECT 1;").unwrap();
        let result = migrate(&mut conn, &bad_dir);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::ChecksumMismatch { version: 2, .. }));
        let tables: String = conn
            .query_row(
                "SELECT group_concat(name) FROM sqlite_master WHERE type = 'table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(tables.contains("schema_migrations"));
        assert!(tables.contains("workspaces"));
        assert!(!tables.contains("0002_fake"));
        drop(conn);
        fs::remove_dir_all(db.parent().unwrap()).unwrap();
        fs::remove_dir_all(bad_dir).unwrap();
    }

    fn temp_dir() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/tmp-bad-migration-{:?}-{:?}",
            std::thread::current().id(),
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn migrations_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/sqlite")
    }
}
