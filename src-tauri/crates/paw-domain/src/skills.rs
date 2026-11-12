//! Skill & Growth assessment entities: Skill, SkillEvidence, Assessment
//! (E6-01, Beta 0.5; PAW-LG-008/009, ADR-015).
//!
//! Ownership per ADR-013/ADR-015: the Learning & Growth domain owns skills,
//! evidence and assessments. Invariants (learning-growth-PRD §3.5–3.7,
//! ADR-015):
//!
//! - skill levels are **user-confirmed facts**, never inferred: an
//!   assessment must be evidence-linked, and no automation may write one
//!   (Agent suggestions stay suggestions — PRD §6);
//! - every assessment snapshots the skill's `scale_version` at assessment
//!   time, so changing the scale semantics never reinterprets history;
//! - evidence points to its source (Artifact/Note/KnowledgeRef/Task/
//!   Project) by opaque id without copying source content;
//! - assessment history is append-only: corrections are new records.
//!
//! Pure module: no SQL, filesystem, OS, or Tauri dependency (ADR-018).

use serde::{Deserialize, Serialize};

use crate::work_management::{WorkspaceId, opaque_id, validate_name};

opaque_id!(SkillId, "Opaque skill identifier (UUIDv7 text).");
opaque_id!(
    SkillEvidenceId,
    "Opaque skill evidence identifier (UUIDv7 text)."
);
opaque_id!(
    AssessmentId,
    "Opaque learning assessment identifier (UUIDv7 text)."
);

/// Domain invariant violations for the skills module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillsError {
    /// A required name was empty or whitespace-only after trimming.
    EmptyName,
    /// A name exceeded its limit after trimming.
    NameTooLong { max: usize },
    /// A free-form text field exceeded its limit.
    TextTooLong { max: usize },
    /// An assessment was constructed without evidence links.
    AssessmentRequiresEvidence,
    /// A skill level outside the scale range (0..=5).
    InvalidLevel { level: u8 },
}

impl std::fmt::Display for SkillsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "name must not be empty"),
            Self::NameTooLong { max } => write!(f, "name exceeds {max} characters"),
            Self::TextTooLong { max } => write!(f, "text exceeds {max} characters"),
            Self::AssessmentRequiresEvidence => {
                write!(
                    f,
                    "an assessment must reference at least one evidence record"
                )
            }
            Self::InvalidLevel { level } => write!(f, "skill level {level} outside 0..=5"),
        }
    }
}

impl std::error::Error for SkillsError {}

impl From<crate::work_management::ValidationError> for SkillsError {
    fn from(e: crate::work_management::ValidationError) -> Self {
        match e {
            crate::work_management::ValidationError::EmptyName => Self::EmptyName,
            crate::work_management::ValidationError::NameTooLong { max } => {
                Self::NameTooLong { max }
            }
            crate::work_management::ValidationError::NotesTooLong { max } => {
                Self::TextTooLong { max }
            }
        }
    }
}

fn validate_text(text: Option<&str>, max: usize) -> Result<Option<String>, SkillsError> {
    match text {
        None => Ok(None),
        Some(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > max {
                return Err(SkillsError::TextTooLong { max });
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

/// What kind of record the evidence points at (PRD §3.6: 证据指向
/// Artifact、Note、KnowledgeRef、Task 或 Project，不默认复制源内容).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceSourceKind {
    Artifact,
    Note,
    KnowledgeRef,
    Task,
    Project,
}

impl EvidenceSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Note => "note",
            Self::KnowledgeRef => "knowledge_ref",
            Self::Task => "task",
            Self::Project => "project",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "artifact" => Self::Artifact,
            "note" => Self::Note,
            "knowledge_ref" => Self::KnowledgeRef,
            "task" => Self::Task,
            "project" => Self::Project,
            _ => return None,
        })
    }
}

/// Evidence type (PRD §3.6: 概念解释、练习、实验、实现、测试验证、
/// 项目交付、技术评审、教学/文档). Strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    ConceptExplanation,
    Practice,
    Experiment,
    Implementation,
    TestVerification,
    ProjectDelivery,
    TechReview,
    TeachingDocumentation,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConceptExplanation => "concept_explanation",
            Self::Practice => "practice",
            Self::Experiment => "experiment",
            Self::Implementation => "implementation",
            Self::TestVerification => "test_verification",
            Self::ProjectDelivery => "project_delivery",
            Self::TechReview => "tech_review",
            Self::TeachingDocumentation => "teaching_documentation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "concept_explanation" => Self::ConceptExplanation,
            "practice" => Self::Practice,
            "experiment" => Self::Experiment,
            "implementation" => Self::Implementation,
            "test_verification" => Self::TestVerification,
            "project_delivery" => Self::ProjectDelivery,
            "tech_review" => Self::TechReview,
            "teaching_documentation" => Self::TeachingDocumentation,
            _ => return None,
        })
    }
}

/// Assessment confidence band (PRD §3.5 shows 置信度 alongside the level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            _ => return None,
        })
    }
}

/// Maximum level on the versioned 0..=5 scale.
pub const MAX_SKILL_LEVEL: u8 = 5;

/// A user-defined skill in the matrix (PRD §3.5). `scale_version` is the
/// version of the assessment-scale semantics this skill's levels are
/// interpreted under; bumping it starts a new assessment series — history
/// keeps the old snapshot (PRD §3.7: 保留评估历史和尺度版本).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub category: String,
    /// Current scale version (>= 1). Assessments snapshot the value at
    /// their creation time.
    pub scale_version: u32,
    pub notes: Option<String>,
}

impl Skill {
    pub fn new(
        id: SkillId,
        workspace_id: WorkspaceId,
        name: &str,
        category: &str,
        notes: Option<&str>,
    ) -> Result<Self, SkillsError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SkillsError::EmptyName);
        }
        if trimmed.chars().count() > 100 {
            return Err(SkillsError::NameTooLong { max: 100 });
        }
        Ok(Self {
            id,
            workspace_id,
            name: trimmed.to_string(),
            category: validate_name(category, 60)?,
            scale_version: 1,
            notes: validate_text(notes, 500)?,
        })
    }

    /// Bump the scale version: starts a new assessment series under new
    /// scale semantics. History is never reinterpreted (ADR-015).
    pub fn bump_scale_version(&mut self) {
        self.scale_version += 1;
    }
}

/// One evidence record pointing at its source (PRD §3.6). `source_id` is
/// the opaque id of the referenced record — cross-domain references stay
/// id-based without FKs (domain-architecture; validated at use time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvidence {
    pub id: SkillEvidenceId,
    pub workspace_id: WorkspaceId,
    pub skill_id: SkillId,
    pub kind: EvidenceKind,
    pub source_kind: EvidenceSourceKind,
    pub source_id: String,
    /// User summary of what the evidence demonstrates (说明).
    pub summary: Option<String>,
}

impl SkillEvidence {
    pub fn new(
        id: SkillEvidenceId,
        workspace_id: WorkspaceId,
        skill_id: SkillId,
        kind: EvidenceKind,
        source_kind: EvidenceSourceKind,
        source_id: &str,
        summary: Option<&str>,
    ) -> Result<Self, SkillsError> {
        let trimmed = source_id.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return Err(SkillsError::TextTooLong { max: 100 });
        }
        Ok(Self {
            id,
            workspace_id,
            skill_id,
            kind,
            source_kind,
            source_id: trimmed.to_string(),
            summary: validate_text(summary, 500)?,
        })
    }
}

/// A user-confirmed assessment (PRD §3.7). Evidence-linked: construction
/// requires at least one evidence id. Append-only — corrections are new
/// assessments; the domain offers no edit path (ADR-015).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assessment {
    pub id: AssessmentId,
    pub workspace_id: WorkspaceId,
    pub skill_id: SkillId,
    /// User-confirmed level on the 0..=5 scale under `scale_version`.
    pub level: u8,
    /// Snapshot of the skill's scale version at assessment time.
    pub scale_version: u32,
    pub confidence: Confidence,
    pub rationale: Option<String>,
    /// Evidence the assessment is grounded in (>= 1, evidence-linked).
    pub evidence_ids: Vec<SkillEvidenceId>,
}

impl Assessment {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AssessmentId,
        workspace_id: WorkspaceId,
        skill_id: SkillId,
        level: u8,
        skill_scale_version: u32,
        confidence: Confidence,
        rationale: Option<&str>,
        evidence_ids: Vec<SkillEvidenceId>,
    ) -> Result<Self, SkillsError> {
        if level > MAX_SKILL_LEVEL {
            return Err(SkillsError::InvalidLevel { level });
        }
        if evidence_ids.is_empty() {
            return Err(SkillsError::AssessmentRequiresEvidence);
        }
        if skill_scale_version == 0 {
            // Scale versions start at 1; zero cannot be a valid snapshot.
            return Err(SkillsError::InvalidLevel { level: 0 });
        }
        Ok(Self {
            id,
            workspace_id,
            skill_id,
            level,
            scale_version: skill_scale_version,
            confidence,
            rationale: validate_text(rationale, 500)?,
            evidence_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceId {
        WorkspaceId::new_v7()
    }

    #[test]
    fn skill_construction_and_scale_bump() {
        let mut skill = Skill::new(
            SkillId::new_v7(),
            ws(),
            "Rust",
            "language",
            Some("systems work"),
        )
        .expect("valid skill");
        assert_eq!(skill.scale_version, 1);
        skill.bump_scale_version();
        skill.bump_scale_version();
        assert_eq!(skill.scale_version, 3);
    }

    #[test]
    fn skill_rejects_empty_and_oversized_names() {
        assert_eq!(
            Skill::new(SkillId::new_v7(), ws(), "  ", "cat", None),
            Err(SkillsError::EmptyName)
        );
        let long = "x".repeat(101);
        assert!(Skill::new(SkillId::new_v7(), ws(), &long, "cat", None).is_err());
    }

    #[test]
    fn evidence_rejects_empty_source_id() {
        assert!(
            SkillEvidence::new(
                SkillEvidenceId::new_v7(),
                ws(),
                SkillId::new_v7(),
                EvidenceKind::Implementation,
                EvidenceSourceKind::Project,
                "  ",
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn assessment_requires_evidence_and_bounded_level() {
        let skill_id = SkillId::new_v7();
        let err = Assessment::new(
            AssessmentId::new_v7(),
            ws(),
            skill_id.clone(),
            3,
            1,
            Confidence::Medium,
            None,
            Vec::new(),
        )
        .expect_err("no evidence");
        assert_eq!(err, SkillsError::AssessmentRequiresEvidence);
        let err = Assessment::new(
            AssessmentId::new_v7(),
            ws(),
            skill_id.clone(),
            6,
            1,
            Confidence::High,
            None,
            vec![SkillEvidenceId::new_v7()],
        )
        .expect_err("level 6");
        assert_eq!(err, SkillsError::InvalidLevel { level: 6 });
        let ok = Assessment::new(
            AssessmentId::new_v7(),
            ws(),
            skill_id,
            5,
            1,
            Confidence::High,
            Some("shipped the codec"),
            vec![SkillEvidenceId::new_v7()],
        )
        .expect("valid assessment");
        assert_eq!(ok.scale_version, 1);
    }

    #[test]
    fn evidence_kinds_roundtrip_through_schema_strings() {
        for kind in [
            EvidenceKind::ConceptExplanation,
            EvidenceKind::Practice,
            EvidenceKind::Experiment,
            EvidenceKind::Implementation,
            EvidenceKind::TestVerification,
            EvidenceKind::ProjectDelivery,
            EvidenceKind::TechReview,
            EvidenceKind::TeachingDocumentation,
        ] {
            assert_eq!(EvidenceKind::parse(kind.as_str()), Some(kind));
        }
        for source in [
            EvidenceSourceKind::Artifact,
            EvidenceSourceKind::Note,
            EvidenceSourceKind::KnowledgeRef,
            EvidenceSourceKind::Task,
            EvidenceSourceKind::Project,
        ] {
            assert_eq!(EvidenceSourceKind::parse(source.as_str()), Some(source));
        }
    }
}
