//! Skills & assessment persistence ports (E6-01; PAW-LG-008/009, ADR-015).

use paw_domain::skills::{
    Assessment, AssessmentId, Skill, SkillEvidence, SkillEvidenceId, SkillId,
};
use paw_domain::work_management::WorkspaceId;

/// Non-leaky persistence errors for the skills surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStoreError {
    NotFound,
    WorkspaceMismatch,
    Invariant(String),
    Storage(String),
}

impl std::fmt::Display for SkillStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "record not found"),
            Self::WorkspaceMismatch => write!(f, "record outside the requested workspace"),
            Self::Invariant(msg) => write!(f, "invariant violated: {msg}"),
            Self::Storage(msg) => write!(f, "storage failure: {msg}"),
        }
    }
}

impl std::error::Error for SkillStoreError {}

/// Skill matrix entries (PRD §3.5). Scale bumps are persisted updates.
pub trait SkillRepository {
    fn insert_skill(&self, skill: &Skill) -> Result<(), SkillStoreError>;
    fn get_skill(&self, id: &SkillId, workspace_id: &WorkspaceId)
    -> Result<Skill, SkillStoreError>;
    fn list_skills(
        &self,
        workspace_id: &WorkspaceId,
        category: Option<&str>,
    ) -> Result<Vec<Skill>, SkillStoreError>;
    /// Persist a rename/re-categorize/notes edit or a scale-version bump.
    fn update_skill(&self, skill: &Skill) -> Result<(), SkillStoreError>;
}

/// Evidence records (PRD §3.6).
pub trait SkillEvidenceRepository {
    fn insert_evidence(&self, evidence: &SkillEvidence) -> Result<(), SkillStoreError>;
    fn get_evidence(
        &self,
        id: &SkillEvidenceId,
        workspace_id: &WorkspaceId,
    ) -> Result<SkillEvidence, SkillStoreError>;
    fn list_evidence(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Vec<SkillEvidence>, SkillStoreError>;
}

/// Assessments (PRD §3.7). Append-only: there is intentionally no update
/// or delete path — corrections are new assessments. Insert persists the
/// assessment and its evidence links atomically.
pub trait AssessmentRepository {
    fn insert_assessment(
        &self,
        assessment: &Assessment,
        latest_confirmed: Option<&Assessment>,
    ) -> Result<(), SkillStoreError>;
    fn get_assessment(
        &self,
        id: &AssessmentId,
        workspace_id: &WorkspaceId,
    ) -> Result<Assessment, SkillStoreError>;
    /// Assessments for one skill, newest first, with their evidence ids.
    fn list_assessments(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Vec<Assessment>, SkillStoreError>;
    /// The most recent user-confirmed assessment of a skill (the matrix
    /// shows this alongside the skill, PRD §3.5).
    fn latest_assessment(
        &self,
        workspace_id: &WorkspaceId,
        skill_id: &SkillId,
    ) -> Result<Option<Assessment>, SkillStoreError>;
}
