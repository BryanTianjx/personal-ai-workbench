//! Knowledge & Context domain entities: KnowledgeRef metadata (PAW-K-001,
//! Alpha 0.2 metadata model). External sources (Lingshu, Hindsight, the
//! web) remain authoritative for their content — the workbench stores only
//! reference metadata and availability state (ADR-013/ADR-016, PRD §4.5).

use serde::{Deserialize, Serialize};

use crate::work_management::{ValidationError, WorkspaceId, opaque_id, validate_name};

opaque_id!(
    KnowledgeRefId,
    "Opaque knowledge reference identifier (UUIDv7 text)."
);

/// Where a reference points; strings match the schema CHECK. Connector
/// integrations (Lingshu/Hindsight) are Alpha 0.2+ and do not change this
/// metadata model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSource {
    Lingshu,
    Hindsight,
    Web,
    Local,
}

impl KnowledgeSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lingshu => "lingshu",
            Self::Hindsight => "hindsight",
            Self::Web => "web",
            Self::Local => "local",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "lingshu" => Self::Lingshu,
            "hindsight" => Self::Hindsight,
            "web" => Self::Web,
            "local" => Self::Local,
            _ => return None,
        })
    }
}

/// Availability of the referenced source (PRD §4.5: 来源失效和降级正确).
/// A stale/unavailable source changes reference availability, never the
/// historical records that reference it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Stale,
    Unavailable,
}

impl Availability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Stale => "stale",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "available" => Self::Available,
            "stale" => Self::Stale,
            "unavailable" => Self::Unavailable,
            _ => return None,
        })
    }
}

/// Reference metadata for an external knowledge asset (PRD §4.5: 来源、
/// 标识、标题、位置、版本/更新时间和可用状态). The external URI is kept
/// separate from the opaque id (cross-domain-data-contracts §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRef {
    pub id: KnowledgeRefId,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub source: KnowledgeSource,
    pub external_uri: Option<String>,
    pub locator: Option<String>,
    pub version_label: Option<String>,
    pub notes: Option<String>,
    pub availability: Availability,
}

impl KnowledgeRef {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: KnowledgeRefId,
        workspace_id: WorkspaceId,
        title: &str,
        source: KnowledgeSource,
        external_uri: Option<&str>,
        locator: Option<&str>,
        version_label: Option<&str>,
        notes: Option<&str>,
    ) -> Result<Self, ValidationError> {
        Ok(Self {
            id,
            workspace_id,
            title: validate_name(title, 200)?,
            source,
            external_uri: external_uri.map(str::to_string),
            locator: locator.map(str::to_string),
            version_label: version_label.map(str::to_string),
            notes: notes.map(str::to_string),
            availability: Availability::Available,
        })
    }
}
