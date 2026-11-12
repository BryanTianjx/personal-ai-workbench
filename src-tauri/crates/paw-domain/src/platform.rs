//! Platform composition domain entities: RoleProfile and
//! WorkspaceComposition (E7-03/E7-04; PAW-INF-008/009, ADR-024/025).
//!
//! Hard boundary (ADR-024/025 R1; cross-ADR review §7): profiles and
//! compositions are **experience/presentation objects** — they never carry
//! grants, credentials, approval tokens, or executable code, and switching
//! them never changes effective authorization. The Data Workspace remains
//! the backend-enforced data/security boundary.
//!
//! Pure module: no SQL, filesystem, OS, or Tauri dependency (ADR-018).

use serde::{Deserialize, Serialize};

use crate::work_management::{WorkspaceId, opaque_id, validate_name};

opaque_id!(ProfileId, "Opaque role profile identifier (UUIDv7 text).");
opaque_id!(
    CompositionId,
    "Opaque workspace composition identifier (UUIDv7 text)."
);

/// Domain invariant violations for the platform composition module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    /// A required name was empty or whitespace-only after trimming.
    EmptyName,
    /// A name/text field exceeded its limit after trimming.
    TooLong { max: usize },
    /// The composition references a plugin/profile that is unknown or not
    /// usable in this workspace — fall back safely (never widen access).
    UnknownReference { reference: String },
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "name must not be empty"),
            Self::TooLong { max } => write!(f, "text exceeds {max} characters"),
            Self::UnknownReference { reference } => {
                write!(f, "unknown or unusable reference: {reference}")
            }
        }
    }
}

impl From<crate::work_management::ValidationError> for CompositionError {
    fn from(e: crate::work_management::ValidationError) -> Self {
        match e {
            crate::work_management::ValidationError::EmptyName => Self::EmptyName,
            crate::work_management::ValidationError::NameTooLong { max }
            | crate::work_management::ValidationError::NotesTooLong { max } => {
                Self::TooLong { max }
            }
        }
    }
}

impl std::error::Error for CompositionError {}

/// A role profile: an experience template (PRD-less platform object,
/// ADR-024). It may reference plugin ids, navigation identifiers, and Agent
/// registration ids. It must NOT contain grants, credentials, approval
/// tokens, or executable code — the struct has no such fields by
/// construction, and import validation strips anything outside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleProfile {
    pub id: ProfileId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    /// Plugin id preferences (ordering, presentation) — never grants.
    pub plugin_preferences: Vec<String>,
    /// Declarative dashboard layout reference (not code).
    pub dashboard_layout: Option<String>,
    /// Agent registration ids the profile recommends.
    pub agent_profile_refs: Vec<String>,
}

impl RoleProfile {
    pub fn new(
        id: ProfileId,
        workspace_id: WorkspaceId,
        name: &str,
        plugin_preferences: Vec<String>,
        dashboard_layout: Option<&str>,
        agent_profile_refs: Vec<String>,
    ) -> Result<Self, CompositionError> {
        for reference in plugin_preferences.iter().chain(agent_profile_refs.iter()) {
            validate_reference(reference)?;
        }
        let layout = validate_optional(dashboard_layout, 200)?;
        Ok(Self {
            id,
            workspace_id,
            name: validate_name(name, 100)?,
            plugin_preferences,
            dashboard_layout: layout,
            agent_profile_refs,
        })
    }
}

/// A workspace composition: presentation/configuration aggregate scoped to
/// an Actor and exactly one existing Data Workspace (ADR-025). It stores
/// references and display preferences only; every command/query continues
/// to resolve the actual Data Workspace and enforce backend authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceComposition {
    pub id: CompositionId,
    pub workspace_id: WorkspaceId,
    pub actor_id: String,
    pub name: String,
    /// Optional role-profile template reference.
    pub profile_ref: Option<ProfileId>,
    /// Plugin selection and ordering preferences (display only).
    pub plugin_selection: Vec<String>,
    /// Declarative navigation entry references (not code).
    pub navigation_config: Option<String>,
}

impl WorkspaceComposition {
    pub fn new(
        id: CompositionId,
        workspace_id: WorkspaceId,
        actor_id: &str,
        name: &str,
        profile_ref: Option<ProfileId>,
        plugin_selection: Vec<String>,
        navigation_config: Option<&str>,
    ) -> Result<Self, CompositionError> {
        if actor_id.trim().is_empty() {
            return Err(CompositionError::EmptyName);
        }
        for reference in &plugin_selection {
            validate_reference(reference)?;
        }
        let navigation = validate_optional(navigation_config, 200)?;
        Ok(Self {
            id,
            workspace_id,
            actor_id: actor_id.trim().to_string(),
            name: validate_name(name, 100)?,
            profile_ref,
            plugin_selection,
            navigation_config: navigation,
        })
    }
}

fn validate_reference(reference: &str) -> Result<(), CompositionError> {
    let ok = !reference.is_empty()
        && reference.len() <= 100
        && reference
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b':'));
    if ok {
        Ok(())
    } else {
        Err(CompositionError::UnknownReference {
            reference: reference.to_string(),
        })
    }
}

fn validate_optional(text: Option<&str>, max: usize) -> Result<Option<String>, CompositionError> {
    match text {
        None => Ok(None),
        Some(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > max {
                return Err(CompositionError::TooLong { max });
            }
            Ok(Some(trimmed.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws() -> WorkspaceId {
        WorkspaceId::new_v7()
    }

    #[test]
    fn profile_holds_references_only() {
        let profile = RoleProfile::new(
            ProfileId::new_v7(),
            ws(),
            "工程师视角",
            vec![
                "core.work_management".to_string(),
                "core.learning_growth".to_string(),
            ],
            Some("default"),
            vec!["agent:123".to_string()],
        )
        .expect("valid profile");
        assert_eq!(profile.plugin_preferences.len(), 2);
        // No grant/credential fields exist by construction — the type is the
        // test (ADR-024 §R1).
    }

    #[test]
    fn profile_rejects_malformed_references() {
        assert!(
            RoleProfile::new(
                ProfileId::new_v7(),
                ws(),
                "bad",
                vec!["DROP TABLE users; --".to_string()],
                None,
                Vec::new()
            )
            .is_err()
        );
    }

    #[test]
    fn composition_scoped_to_actor_and_workspace() {
        let composition = WorkspaceComposition::new(
            CompositionId::new_v7(),
            ws(),
            "local-user",
            "专注模式",
            None,
            vec!["core.work_management".to_string()],
            None,
        )
        .expect("valid composition");
        assert_eq!(composition.actor_id, "local-user");
        assert!(
            WorkspaceComposition::new(
                CompositionId::new_v7(),
                ws(),
                "  ",
                "bad actor",
                None,
                Vec::new(),
                None
            )
            .is_err()
        );
    }
}
