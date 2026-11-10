//! Scoped FTS5 search service (E2-03) plus the index-maintenance helpers
//! the repositories call inside the same transaction as their source change.
//!
//! Contract (data-architecture §7): the FTS5 tables are rebuildable
//! projections; every query enforces the workspace predicate; hits join back
//! to the authoritative tables so deleted records never surface.

use rusqlite::{Connection, params};

use paw_application::ports::search::{SearchError, SearchHit, SearchKind, SearchPort};
use paw_domain::work_management::WorkspaceId;

/// Index one project row (same connection/transaction as the source write).
pub fn index_project(conn: &Connection, project: &paw_domain::work_management::Project) {
    let _ = conn.execute(
        "INSERT INTO fts_projects (name, description, workspace_id, project_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            segment_cjk(&project.name),
            project.description.as_deref().map(segment_cjk),
            project.workspace_id.as_str(),
            project.id.as_str(),
        ],
    );
}

/// Remove a project's index rows.
pub fn deindex_project(conn: &Connection, project_id: &str) {
    let _ = conn.execute(
        "DELETE FROM fts_projects WHERE project_id = ?1",
        params![project_id],
    );
}

/// Index one task row (same connection/transaction as the source write).
pub fn index_task(conn: &Connection, task: &paw_domain::work_management::Task) {
    let _ = conn.execute(
        "INSERT INTO fts_tasks (title, notes, workspace_id, task_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            segment_cjk(&task.title),
            task.notes.as_deref().map(segment_cjk),
            task.workspace_id.as_str(),
            task.id.as_str(),
        ],
    );
}

/// Remove a task's index rows.
pub fn deindex_task(conn: &Connection, task_id: &str) {
    let _ = conn.execute("DELETE FROM fts_tasks WHERE task_id = ?1", params![task_id]);
}

/// Is `c` a CJK ideograph? The default FTS5 unicode61 tokenizer does not
/// segment CJK text, so the index and the query both space-separate CJK
/// characters (deterministic, no external tokenizer dependency). Phrase
/// quoting keeps a multi-character query token adjacent after segmentation.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3400}'..='\u{9FFF}'      // CJK unified ideographs + ext A
        | '\u{F900}'..='\u{FAFF}'    // CJK compatibility ideographs
        | '\u{20000}'..='\u{2FA1F}'  // extensions B..F
    )
}

/// Space-separate consecutive CJK characters: "边缘计算" → "边 缘 计 算".
fn segment_cjk(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut prev_cjk = false;
    for c in text.chars() {
        let cjk = is_cjk(c);
        if cjk && prev_cjk {
            out.push(' ');
        }
        out.push(c);
        prev_cjk = cjk;
    }
    out
}

/// Escape user input into a safe FTS5 MATCH expression: whitespace-split
/// tokens, CJK-segmented, each double-quoted (FTS5 phrase), implicitly
/// ANDed. Tokens containing double quotes are dropped, so user input can
/// never inject query syntax. Returns `None` when nothing searchable
/// remains.
fn sanitize_query(query: &str) -> Option<String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .filter(|t| !t.is_empty() && !t.contains('"'))
        .map(|t| format!("\"{}\"", segment_cjk(t)))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

/// SQLite-backed scoped search over the FTS5 projections.
pub struct SqliteSearchService {
    conn: Connection,
}

impl SqliteSearchService {
    pub fn open(path: &std::path::Path) -> Result<Self, crate::sqlite::Error> {
        Ok(Self {
            conn: crate::sqlite::open(path)?,
        })
    }

    /// Rebuild both projections from the authoritative tables (idempotent;
    /// used after upgrades and repairable at any time). Re-indexing runs
    /// through the same CJK segmentation as live writes.
    pub fn rebuild(&self) -> Result<(), SearchError> {
        self.conn
            .execute("DELETE FROM fts_projects", [])
            .map_err(|e| SearchError::Invariant(e.to_string()))?;
        self.conn
            .execute("DELETE FROM fts_tasks", [])
            .map_err(|e| SearchError::Invariant(e.to_string()))?;

        let mut stmt = self
            .conn
            .prepare("SELECT id, workspace_id, name, description FROM projects")
            .map_err(|e| SearchError::Invariant(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| SearchError::Invariant(e.to_string()))?;
        for row in rows {
            let (id, workspace_id, name, description) =
                row.map_err(|e| SearchError::Invariant(e.to_string()))?;
            let _ = self.conn.execute(
                "INSERT INTO fts_projects (name, description, workspace_id, project_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    segment_cjk(&name),
                    description.as_deref().map(segment_cjk),
                    workspace_id,
                    id,
                ],
            );
        }

        let mut stmt = self
            .conn
            .prepare("SELECT id, workspace_id, title, notes FROM tasks")
            .map_err(|e| SearchError::Invariant(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| SearchError::Invariant(e.to_string()))?;
        for row in rows {
            let (id, workspace_id, title, notes) =
                row.map_err(|e| SearchError::Invariant(e.to_string()))?;
            let _ = self.conn.execute(
                "INSERT INTO fts_tasks (title, notes, workspace_id, task_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    segment_cjk(&title),
                    notes.as_deref().map(segment_cjk),
                    workspace_id,
                    id,
                ],
            );
        }
        Ok(())
    }
}

impl SearchPort for SqliteSearchService {
    fn search(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let Some(match_expr) = sanitize_query(query) else {
            return Err(SearchError::InvalidQuery);
        };
        let limit = limit.clamp(1, 100) as i64;

        let mut hits: Vec<SearchHit> = Vec::new();

        // Projects: join back to the authoritative table for existence and
        // the display name; the FTS workspace predicate scopes the query.
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT p.id, p.name FROM fts_projects f
                 JOIN projects p ON p.id = f.project_id AND p.workspace_id = f.workspace_id
                 WHERE fts_projects MATCH ?2 AND f.workspace_id = ?1
                 ORDER BY rank LIMIT ?3",
                )
                .map_err(|e| SearchError::Invariant(e.to_string()))?;
            let rows = stmt
                .query_map(params![workspace_id.as_str(), match_expr, limit], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| SearchError::Invariant(e.to_string()))?;
            for row in rows {
                let (id, title) = row.map_err(|e| SearchError::Invariant(e.to_string()))?;
                hits.push(SearchHit {
                    kind: SearchKind::Project,
                    id,
                    workspace_id: workspace_id.clone(),
                    title,
                });
            }
        }

        // Tasks (fill up to the shared limit).
        let remaining = limit - hits.len() as i64;
        if remaining > 0 {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT t.id, t.title FROM fts_tasks f
                 JOIN tasks t ON t.id = f.task_id AND t.workspace_id = f.workspace_id
                 WHERE fts_tasks MATCH ?2 AND f.workspace_id = ?1
                 ORDER BY rank LIMIT ?3",
                )
                .map_err(|e| SearchError::Invariant(e.to_string()))?;
            let rows = stmt
                .query_map(
                    params![workspace_id.as_str(), match_expr, remaining],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .map_err(|e| SearchError::Invariant(e.to_string()))?;
            for row in rows {
                let (id, title) = row.map_err(|e| SearchError::Invariant(e.to_string()))?;
                hits.push(SearchHit {
                    kind: SearchKind::Task,
                    id,
                    workspace_id: workspace_id.clone(),
                    title,
                });
            }
        }

        Ok(hits)
    }
}
