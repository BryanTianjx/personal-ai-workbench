//! Learning & Growth domain entities: Roadmap, Plan, Item, Session
//! (Alpha 0.2 basic lifecycle, PAW-LG-001～006).
//!
//! Ownership per ADR-013/ADR-014 and the Learning & Growth PRD: Learning
//! owns these records; Project/Task remain Work Management-owned; external
//! sources remain Knowledge-owned. Sessions are historical records — they
//! are append-only and never cascade-deleted because a referenced record
//! goes away (cross-domain-data-contracts §7).

use serde::{Deserialize, Serialize};

use crate::work_management::{TASK_NOTES_MAX, ValidationError, opaque_id, validate_name};

opaque_id!(
    RoadmapId,
    "Opaque learning roadmap identifier (UUIDv7 text)."
);
opaque_id!(PlanId, "Opaque learning plan identifier (UUIDv7 text).");
opaque_id!(ItemId, "Opaque learning item identifier (UUIDv7 text).");
opaque_id!(
    SessionId,
    "Opaque learning session identifier (UUIDv7 text)."
);

/// Plan lifecycle states (PRD §3.2); strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Archived,
}

impl PlanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "draft" => Self::Draft,
            "active" => Self::Active,
            "paused" => Self::Paused,
            "completed" => Self::Completed,
            "archived" => Self::Archived,
            _ => return None,
        })
    }

    /// Free transitions for the Alpha 0.2 slice — completion means the user
    /// confirmed the plan's goals, never a skill-level change.
    pub fn can_transition_to(self, to: Self) -> bool {
        !self.is_terminal() || to == Self::Archived
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Archived)
    }
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Item kinds (PRD §3.3); strings match the schema CHECK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Concept,
    Reading,
    Exercise,
    Experiment,
    Implementation,
    Review,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Concept => "concept",
            Self::Reading => "reading",
            Self::Exercise => "exercise",
            Self::Experiment => "experiment",
            Self::Implementation => "implementation",
            Self::Review => "review",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "concept" => Self::Concept,
            "reading" => Self::Reading,
            "exercise" => Self::Exercise,
            "experiment" => Self::Experiment,
            "implementation" => Self::Implementation,
            "review" => Self::Review,
            _ => return None,
        })
    }
}

/// Item lifecycle states; strings match the schema CHECK. Progress is
/// bookkeeping only — time and completion are never evidence of skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStatus {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl ItemStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "todo" => Self::Todo,
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }

    /// Same machine as TaskStatus (mirrored semantics, separate enum:
    /// cross-domain-data-contracts §2 — each domain owns its state).
    pub fn can_transition_to(self, to: Self) -> bool {
        use ItemStatus::*;
        matches!(
            (self, to),
            (Todo, InProgress)
                | (Todo, Cancelled)
                | (InProgress, Done)
                | (InProgress, Todo)
                | (InProgress, Cancelled)
        )
    }
}

impl std::fmt::Display for ItemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A long-term learning direction (PRD §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roadmap {
    pub id: RoadmapId,
    pub workspace_id: WorkspaceScope,
    pub name: String,
    pub description: Option<String>,
}

/// Workspace binding reused across learning entities (typed alias keeps the
/// composite-FK intent visible in signatures).
pub type WorkspaceScope = crate::work_management::WorkspaceId;

impl Roadmap {
    pub fn new(
        id: RoadmapId,
        workspace_id: WorkspaceScope,
        name: &str,
        description: Option<&str>,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            id,
            workspace_id,
            name: validate_name(name, 200)?,
            description: description.map(str::to_string),
        })
    }
}

/// A staged learning plan under a roadmap (PRD §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    pub workspace_id: WorkspaceScope,
    pub roadmap_id: RoadmapId,
    pub name: String,
    pub description: Option<String>,
    pub status: PlanStatus,
}

impl Plan {
    pub fn new(
        id: PlanId,
        workspace_id: WorkspaceScope,
        roadmap_id: RoadmapId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            id,
            workspace_id,
            roadmap_id,
            name: validate_name(name, 200)?,
            description: description.map(str::to_string),
            status: PlanStatus::Draft,
        })
    }
}

/// A learnable/practicable/verifiable target under a plan (PRD §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub workspace_id: WorkspaceScope,
    pub plan_id: PlanId,
    pub kind: ItemKind,
    pub title: String,
    pub expected_outcome: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub status: ItemStatus,
}

impl Item {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ItemId,
        workspace_id: WorkspaceScope,
        plan_id: PlanId,
        kind: ItemKind,
        title: &str,
        expected_outcome: Option<&str>,
        acceptance_criteria: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            id,
            workspace_id,
            plan_id,
            kind,
            title: validate_name(title, 200)?,
            expected_outcome: expected_outcome.map(str::to_string),
            acceptance_criteria: acceptance_criteria.map(str::to_string),
            notes: match notes {
                None => None,
                Some(text) if text.trim().chars().count() > TASK_NOTES_MAX => {
                    return Err(ValidationError::NotesTooLong {
                        max: TASK_NOTES_MAX,
                    });
                }
                Some(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
            },
            status: ItemStatus::Todo,
        })
    }
}

/// One actually-occurred learning activity (PRD §3.4). Append-only: a
/// Session is a historical record and is never overwritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub workspace_id: WorkspaceScope,
    pub item_id: ItemId,
    ///关联的 Work Management 任务（action_step；Task 仍归 Work Management）。
    pub task_id: Option<crate::work_management::TaskId>,
    /// 关联的项目（practice_context；Project 仍归 Work Management）。
    pub project_id: Option<crate::work_management::ProjectId>,
    pub method: Option<String>,
    /// Optional duration in minutes.
    pub duration_minutes: Option<i64>,
    pub summary: Option<String>,
    pub difficulty: Option<String>,
    pub takeaway: Option<String>,
    pub next_step: Option<String>,
    pub occurred_at: i64,
}

/// Free-text learning fields capped at the notes limit, shared validation.
impl Session {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: SessionId,
        workspace_id: WorkspaceScope,
        item_id: ItemId,
        task_id: Option<crate::work_management::TaskId>,
        project_id: Option<crate::work_management::ProjectId>,
        method: Option<&str>,
        duration_minutes: Option<i64>,
        summary: Option<&str>,
        difficulty: Option<&str>,
        takeaway: Option<&str>,
        next_step: Option<&str>,
        occurred_at: i64,
    ) -> Result<Self, ValidationError> {
        let duration_ok = duration_minutes.map(|d| d >= 0).unwrap_or(true);
        if !duration_ok {
            return Err(ValidationError::NotesTooLong {
                max: TASK_NOTES_MAX,
            });
        }
        Ok(Self {
            id,
            workspace_id,
            item_id,
            task_id,
            project_id,
            method: method.map(str::to_string),
            duration_minutes,
            summary: summary.map(str::to_string),
            difficulty: difficulty.map(str::to_string),
            takeaway: takeaway.map(str::to_string),
            next_step: next_step.map(str::to_string),
            occurred_at,
        })
    }
}
