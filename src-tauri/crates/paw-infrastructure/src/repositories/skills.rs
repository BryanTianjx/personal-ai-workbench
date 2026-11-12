//! SQLite adapters for the skills/assessment surface (E6-01, PAW-LG-008/009).
//!
//! Assessments are append-only (no UPDATE path); inserting one persists the
//! evidence link rows atomically. Every query carries a workspace predicate.

use std::path::Path;

use paw_application::ports::skill_repository::{
    AssessmentRepository, SkillEvidenceRepository, SkillRepository, SkillStoreError,
};
use paw_domain::skills::{
    Assessment, AssessmentId, Confidence, EvidenceKind, EvidenceSourceKind, Skill, SkillEvidence,
    SkillEvidenceId, SkillId,
};
use paw_domain::work_management::{ParseError, WorkspaceId};

use crate::sqlite::{self, Error};

enum RepoError {
    Sql(rusqlite::Error),
    Now(sqlite::Error),
    Parse(ParseError),
    NotFound,
}

impl From<rusqlite::Error> for RepoError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

impl From<Error> for RepoError {
    fn from(e: Error) -> Self {
        Self::Now(e)
    }
}

impl From<ParseError> for RepoError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<RepoError> for SkillStoreError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::Sql(e) => SkillStoreError::Invariant(e.to_string()),
            RepoError::Now(e) => SkillStoreError::Storage(e.to_string()),
            RepoError::Parse(e) => SkillStoreError::Invariant(e.to_string()),
            RepoError::NotFound => SkillStoreError::NotFound,
        }
    }
}

fn store_error(e: RepoError) -> SkillStoreError {
    e.into()
}

fn query_one<T>(
    stmt: rusqlite::Result<rusqlite::Statement<'_>>,
    params: impl rusqlite::Params,
    map: impl Fn(&rusqlite::Row<'_>) -> Result<T, RepoError>,
) -> Result<T, RepoError> {
    let mut stmt = stmt?;
    let mut rows = stmt.query(params)?;
    let Some(row) = rows.next()? else {
        return Err(RepoError::NotFound);
    };
    map(row)
}

fn query_all<T>(
    stmt: rusqlite::Result<rusqlite::Statement<'_>>,
    params: impl rusqlite::Params,
    map: impl Fn(&rusqlite::Row<'_>) -> Result<T, RepoError>,
) -> Result<Vec<T>, RepoError> {
    let mut stmt = stmt?;
    let mut rows = stmt.query(params)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(map(row)?);
    }
    Ok(out)
}

fn now() -> Result<i64, SkillStoreError> {
    sqlite::now_ms().map_err(|e| SkillStoreError::Storage(e.to_string()))
}

pub struct SqliteSkillRepository {
    conn: rusqlite::Connection,
}

impl SqliteSkillRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_skill(row: &rusqlite::Row<'_>) -> Result<Skill, RepoError> {
    Ok(Skill {
        id: SkillId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        name: row.get("name")?,
        category: row.get("category")?,
        scale_version: row.get::<_, i64>("scale_version")? as u32,
        notes: row.get("notes")?,
    })
}

impl SkillRepository for SqliteSkillRepository {
    fn insert_skill(&self, skill: &Skill) -> Result<(), SkillStoreError> {
        self.conn
            .execute(
                "INSERT INTO skills (id, workspace_id, name, category, scale_version, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    skill.id.as_str(),
                    skill.workspace_id.as_str(),
                    skill.name,
                    skill.category,
                    skill.scale_version,
                    skill.notes,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_skill(
        &self,
        id: &SkillId,
        workspace_id: &WorkspaceId,
    ) -> Result<Skill, SkillStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, name, category, scale_version, notes
             FROM skills WHERE id = ?1",
        );
        let found =
            query_one(stmt, rusqlite::params![id.as_str()], row_to_skill).map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(SkillStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_skills(
        &self,
        workspace_id: &WorkspaceId,
        category: Option<&str>,
    ) -> Result<Vec<Skill>, SkillStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, name, category, scale_version, notes
             FROM skills
             WHERE workspace_id = ?1 AND (?2 IS NULL OR category = ?2)
             ORDER BY category, name, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), category],
            row_to_skill,
        )
        .map_err(store_error)
    }

    fn update_skill(&self, skill: &Skill) -> Result<(), SkillStoreError> {
        let changed = self
            .conn
            .execute(
                "UPDATE skills SET name = ?1, category = ?2, scale_version = ?3, notes = ?4
                 WHERE id = ?5 AND workspace_id = ?6",
                rusqlite::params![
                    skill.name,
                    skill.category,
                    skill.scale_version,
                    skill.notes,
                    skill.id.as_str(),
                    skill.workspace_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        if changed == 0 {
            return Err(SkillStoreError::NotFound);
        }
        Ok(())
    }
}

pub struct SqliteSkillEvidenceRepository {
    conn: rusqlite::Connection,
}

impl SqliteSkillEvidenceRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_evidence(row: &rusqlite::Row<'_>) -> Result<SkillEvidence, RepoError> {
    let kind: String = row.get("kind")?;
    let source_kind: String = row.get("source_kind")?;
    Ok(SkillEvidence {
        id: SkillEvidenceId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        skill_id: SkillId::new(row.get::<_, String>("skill_id")?)?,
        kind: EvidenceKind::parse(&kind)
            .expect("evidence kind values are constrained by the schema CHECK"),
        source_kind: EvidenceSourceKind::parse(&source_kind)
            .expect("source kind values are constrained by the schema CHECK"),
        source_id: row.get("source_id")?,
        summary: row.get("summary")?,
    })
}

impl SkillEvidenceRepository for SqliteSkillEvidenceRepository {
    fn insert_evidence(&self, evidence: &SkillEvidence) -> Result<(), SkillStoreError> {
        let now = now()?;
        self.conn
            .execute(
                "INSERT INTO skill_evidence
                 (id, workspace_id, skill_id, kind, source_kind, source_id, summary, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    evidence.id.as_str(),
                    evidence.workspace_id.as_str(),
                    evidence.skill_id.as_str(),
                    evidence.kind.as_str(),
                    evidence.source_kind.as_str(),
                    evidence.source_id,
                    evidence.summary,
                    now,
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        Ok(())
    }

    fn get_evidence(
        &self,
        id: &SkillEvidenceId,
        workspace_id: &WorkspaceId,
    ) -> Result<SkillEvidence, SkillStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, skill_id, kind, source_kind, source_id, summary
             FROM skill_evidence WHERE id = ?1",
        );
        let found = query_one(stmt, rusqlite::params![id.as_str()], row_to_evidence)
            .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(SkillStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_evidence(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Vec<SkillEvidence>, SkillStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, skill_id, kind, source_kind, source_id, summary
             FROM skill_evidence WHERE workspace_id = ?1 AND skill_id = ?2
             ORDER BY created_at DESC, id",
        );
        query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), skill_id.as_str()],
            row_to_evidence,
        )
        .map_err(store_error)
    }
}

pub struct SqliteAssessmentRepository {
    conn: rusqlite::Connection,
}

impl SqliteAssessmentRepository {
    pub fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            conn: sqlite::open(path)?,
        })
    }
}

fn row_to_assessment(
    row: &rusqlite::Row<'_>,
    evidence_ids: Vec<SkillEvidenceId>,
) -> Result<Assessment, RepoError> {
    let confidence: String = row.get("confidence")?;
    Ok(Assessment {
        id: AssessmentId::new(row.get::<_, String>("id")?)?,
        workspace_id: WorkspaceId::new(row.get::<_, String>("workspace_id")?)?,
        skill_id: SkillId::new(row.get::<_, String>("skill_id")?)?,
        level: row.get::<_, i64>("level")? as u8,
        scale_version: row.get::<_, i64>("scale_version")? as u32,
        confidence: Confidence::parse(&confidence)
            .expect("confidence values are constrained by the schema CHECK"),
        rationale: row.get("rationale")?,
        evidence_ids,
    })
}

fn load_evidence_ids(
    conn: &rusqlite::Connection,
    assessment_id: &str,
    workspace_id: &WorkspaceId,
) -> Result<Vec<SkillEvidenceId>, RepoError> {
    let mut stmt = conn.prepare(
        "SELECT evidence_id FROM learning_assessment_evidence
         WHERE assessment_id = ?1 AND workspace_id = ?2 ORDER BY evidence_id",
    )?;
    let mut rows = stmt.query(rusqlite::params![assessment_id, workspace_id.as_str()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(SkillEvidenceId::new(row.get::<_, String>(0)?)?);
    }
    Ok(out)
}

impl AssessmentRepository for SqliteAssessmentRepository {
    /// Append-only insert: assessment + evidence links in one transaction.
    /// `latest_confirmed` is accepted for interface stability (the caller
    /// may pass the previous head assessment for audit purposes); the
    /// record itself is independent — history is never rewritten.
    fn insert_assessment(
        &self,
        assessment: &Assessment,
        _latest_confirmed: Option<&Assessment>,
    ) -> Result<(), SkillStoreError> {
        let now = now()?;
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        tx.execute(
            "INSERT INTO learning_assessments
             (id, workspace_id, skill_id, level, scale_version, confidence, rationale, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                assessment.id.as_str(),
                assessment.workspace_id.as_str(),
                assessment.skill_id.as_str(),
                assessment.level,
                assessment.scale_version,
                assessment.confidence.as_str(),
                assessment.rationale,
                now,
            ],
        )
        .map_err(RepoError::Sql)
        .map_err(store_error)?;
        for evidence_id in &assessment.evidence_ids {
            tx.execute(
                "INSERT INTO learning_assessment_evidence
                 (assessment_id, workspace_id, evidence_id)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    assessment.id.as_str(),
                    assessment.workspace_id.as_str(),
                    evidence_id.as_str(),
                ],
            )
            .map_err(RepoError::Sql)
            .map_err(store_error)?;
        }
        tx.commit().map_err(RepoError::Sql).map_err(store_error)?;
        Ok(())
    }

    fn get_assessment(
        &self,
        id: &AssessmentId,
        workspace_id: &WorkspaceId,
    ) -> Result<Assessment, SkillStoreError> {
        let evidence_ids =
            load_evidence_ids(&self.conn, id.as_str(), workspace_id).map_err(store_error)?;
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, skill_id, level, scale_version, confidence, rationale
             FROM learning_assessments WHERE id = ?1",
        );
        let found = query_one(stmt, rusqlite::params![id.as_str()], |row| {
            row_to_assessment(row, evidence_ids.clone())
        })
        .map_err(store_error)?;
        if found.workspace_id != *workspace_id {
            return Err(SkillStoreError::WorkspaceMismatch);
        }
        Ok(found)
    }

    fn list_assessments(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Vec<Assessment>, SkillStoreError> {
        let stmt = self.conn.prepare(
            "SELECT id, workspace_id, skill_id, level, scale_version, confidence, rationale
             FROM learning_assessments WHERE workspace_id = ?1 AND skill_id = ?2
             ORDER BY created_at DESC, id",
        );
        let rows = query_all(
            stmt,
            rusqlite::params![workspace_id.as_str(), skill_id.as_str()],
            |row| {
                let id: String = row.get("id")?;
                let evidence_ids = load_evidence_ids(&self.conn, &id, workspace_id)?;
                row_to_assessment(row, evidence_ids)
            },
        )
        .map_err(store_error)?;
        Ok(rows)
    }

    fn latest_assessment(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Option<Assessment>, SkillStoreError> {
        let all = self.list_assessments(workspace_id, skill_id)?;
        Ok(all.into_iter().next())
    }
}
