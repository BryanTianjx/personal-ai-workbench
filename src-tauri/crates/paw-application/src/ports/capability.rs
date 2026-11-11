//! Capability descriptor port and the metadata-only capability registry
//! (E5-01, PAW-INF-005; ADR-026).
//!
//! Each first-party domain publishes explicitly versioned application
//! capabilities routed to its owning use cases. The registry indexes
//! descriptors and resolves supported versions; it is **not** a general
//! service locator and never returns an arbitrary callable — dispatch is
//! mediated through the owning domain's use case and the Application
//! Coordination path (ADR-026 R1 §R1).
//!
//! Descriptor metadata is advisory input to governance: required scopes,
//! risk classes and approval hints are **not grants**. Effective
//! authorization is evaluated by backend policy on each protected side
//! effect.

use std::collections::HashMap;

/// The PAW business domains that own capabilities (registration metadata;
/// ownership itself stays with the domain modules, ADR-027).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityDomain {
    WorkManagement,
    LearningGrowth,
    KnowledgeContext,
    AiCollaboration,
    SystemSecurity,
    EngineeringDelivery,
}

impl CapabilityDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkManagement => "work_management",
            Self::LearningGrowth => "learning_growth",
            Self::KnowledgeContext => "knowledge_context",
            Self::AiCollaboration => "ai_collaboration",
            Self::SystemSecurity => "system_security",
            Self::EngineeringDelivery => "engineering_delivery",
        }
    }
}

/// Risk classification attached to a capability descriptor. `High` forces
/// the approval path regardless of grant state (authorization model §4.2);
/// the catalog is deliberately coarse in E5 (spec §12.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskClass {
    Low,
    Medium,
    High,
}

/// Idempotency semantics declared per capability (ADR-026 R1): retries of
/// `NonIdempotent` capabilities are blocked while the prior outcome is
/// uncertain (ADR-019 OutcomeUnknown rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Idempotency {
    Idempotent,
    NonIdempotent,
    Unknown,
}

/// Offline behavior declared per capability (ADR-005): built-in capabilities
/// and core flows must define what happens without network services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflinePolicy {
    /// Works fully offline; no provider involvement.
    LocalOnly,
    /// Durable envelope defers the external effect until connectivity.
    Deferred,
    /// Requires a live provider; offline means explicit unavailability.
    Unavailable,
}

/// Approval-policy hint. Declarative only: the backend approval decision is
/// computed from policy + risk class, never from this hint alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalHint {
    None,
    Required,
    Conditional,
}

/// Declarative scope requirements (grant dimensions an invocation must
/// satisfy). Metadata for governance and display, not a grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeRequirement {
    /// Capability keys the presenting grant must include.
    pub required_capabilities: Vec<String>,
    /// Tool keys the presenting grant must include (empty = no tool).
    pub required_tools: Vec<String>,
}

impl ScopeRequirement {
    pub fn none() -> Self {
        Self {
            required_capabilities: Vec::new(),
            required_tools: Vec::new(),
        }
    }
}

/// Versioned capability contract identity (`major.minor.patch`). Unknown
/// major versions fail closed (ADR-028 §4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ContractVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn as_str(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A published capability descriptor (`E5-implementation-specification` §4).
/// First-party and static: descriptors are built once at startup and are
/// immutable thereafter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    /// Stable id, `domain.action` namespace (validated downstream).
    pub id: String,
    pub owner: CapabilityDomain,
    pub contract_version: ContractVersion,
    pub required_scope: ScopeRequirement,
    pub risk_class: RiskClass,
    pub idempotency: Idempotency,
    pub offline_behavior: OfflinePolicy,
    pub approval_hint: ApprovalHint,
    /// Human-readable summary shown in approval surfaces (no secrets).
    pub summary: String,
}

/// Registry resolution result. Deliberately not a callable: dispatch goes
/// through the owning domain's use case (ADR-026 R1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Supported(Box<CapabilityDescriptor>),
    /// The capability exists but the requested contract version is not
    /// published (fail closed).
    UnsupportedVersion {
        available: Vec<ContractVersion>,
    },
    /// No such capability id.
    UnknownCapability,
}

/// Registry errors at registration time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// The same (id, contract_version) was registered twice.
    DuplicateRegistration { id: String, version: String },
    /// The capability id violates the `domain.action` namespace.
    InvalidCapabilityId(String),
    /// A `High` risk capability must carry a forced approval hint.
    HighRiskWithoutApproval { id: String },
}

/// Metadata-only capability registry (E5-01). First-party descriptors are
/// statically registered at application startup; there is no dynamic
/// loading, marketplace, or runtime mutation (ADR-023/ADR-028 scope).
#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    by_id: HashMap<String, Vec<CapabilityDescriptor>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a descriptor. Validation failures reject the registration
    /// (and a duplicate (id, version) pair is a programming error).
    pub fn register(&mut self, descriptor: CapabilityDescriptor) -> Result<(), RegistryError> {
        if descriptor.id.is_empty()
            || !descriptor
                .id
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_'))
            || !descriptor.id.contains('.')
        {
            return Err(RegistryError::InvalidCapabilityId(descriptor.id));
        }
        if descriptor.risk_class == RiskClass::High
            && descriptor.approval_hint == ApprovalHint::None
        {
            return Err(RegistryError::HighRiskWithoutApproval {
                id: descriptor.id.clone(),
            });
        }
        let versions = self.by_id.entry(descriptor.id.clone()).or_default();
        if versions
            .iter()
            .any(|d| d.contract_version == descriptor.contract_version)
        {
            return Err(RegistryError::DuplicateRegistration {
                id: descriptor.id,
                version: descriptor.contract_version.to_string(),
            });
        }
        versions.push(descriptor);
        Ok(())
    }

    /// Resolve a capability id at an exact contract version. Unknown major
    /// versions fail closed like any unsupported version (ADR-028 §4.3).
    pub fn resolve(&self, id: &str, version: ContractVersion) -> Resolution {
        match self.by_id.get(id) {
            None => Resolution::UnknownCapability,
            Some(versions) => {
                if let Some(found) = versions
                    .iter()
                    .find(|d| d.contract_version == version)
                    .cloned()
                {
                    Resolution::Supported(Box::new(found))
                } else {
                    Resolution::UnsupportedVersion {
                        available: versions.iter().map(|d| d.contract_version).collect(),
                    }
                }
            }
        }
    }

    /// All published descriptors (used by conformance tests and the
    /// desktop capability listing).
    pub fn descriptors(&self) -> impl Iterator<Item = &CapabilityDescriptor> {
        self.by_id.values().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str, version: ContractVersion, risk: RiskClass) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: id.to_string(),
            owner: CapabilityDomain::WorkManagement,
            contract_version: version,
            required_scope: ScopeRequirement::none(),
            risk_class: risk,
            idempotency: Idempotency::Idempotent,
            offline_behavior: OfflinePolicy::LocalOnly,
            approval_hint: if risk == RiskClass::High {
                ApprovalHint::Required
            } else {
                ApprovalHint::None
            },
            summary: "test capability".to_string(),
        }
    }

    #[test]
    fn register_and_resolve_exact_version() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(descriptor(
                "work.task.create",
                ContractVersion::new(1, 0, 0),
                RiskClass::Low,
            ))
            .expect("valid registration");
        assert!(matches!(
            registry.resolve("work.task.create", ContractVersion::new(1, 0, 0)),
            Resolution::Supported(_)
        ));
    }

    #[test]
    fn unsupported_version_fails_closed() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(descriptor(
                "work.task.create",
                ContractVersion::new(1, 0, 0),
                RiskClass::Low,
            ))
            .expect("valid registration");
        assert!(matches!(
            registry.resolve("work.task.create", ContractVersion::new(2, 0, 0)),
            Resolution::UnsupportedVersion { .. }
        ));
        assert!(matches!(
            registry.resolve("work.task.create", ContractVersion::new(1, 1, 0)),
            Resolution::UnsupportedVersion { .. }
        ));
    }

    #[test]
    fn unknown_capability_fails_closed() {
        let registry = CapabilityRegistry::new();
        assert_eq!(
            registry.resolve("no.such.capability", ContractVersion::new(1, 0, 0)),
            Resolution::UnknownCapability
        );
    }

    #[test]
    fn duplicate_id_version_rejected() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(descriptor(
                "work.task",
                ContractVersion::new(1, 0, 0),
                RiskClass::Low,
            ))
            .expect("first registration");
        assert_eq!(
            registry.register(descriptor(
                "work.task",
                ContractVersion::new(1, 0, 0),
                RiskClass::Low
            )),
            Err(RegistryError::DuplicateRegistration {
                id: "work.task".to_string(),
                version: "1.0.0".to_string(),
            })
        );
        // A different version of the same id is a new published contract.
        registry
            .register(descriptor(
                "work.task",
                ContractVersion::new(1, 1, 0),
                RiskClass::Low,
            ))
            .expect("second version registration");
    }

    #[test]
    fn invalid_capability_ids_rejected() {
        let mut registry = CapabilityRegistry::new();
        for bad in ["nodot", "", "Work.Task", "has space"] {
            assert_eq!(
                registry.register(descriptor(
                    bad,
                    ContractVersion::new(1, 0, 0),
                    RiskClass::Low
                )),
                Err(RegistryError::InvalidCapabilityId(bad.to_string()))
            );
        }
    }

    #[test]
    fn high_risk_requires_approval_hint() {
        let mut registry = CapabilityRegistry::new();
        let mut high = descriptor(
            "work.shell.run",
            ContractVersion::new(1, 0, 0),
            RiskClass::High,
        );
        high.approval_hint = ApprovalHint::None;
        assert_eq!(
            registry.register(high),
            Err(RegistryError::HighRiskWithoutApproval {
                id: "work.shell.run".to_string(),
            })
        );
    }

    #[test]
    fn multiple_versions_listed_on_unsupported_resolution() {
        let mut registry = CapabilityRegistry::new();
        registry
            .register(descriptor(
                "work.task",
                ContractVersion::new(1, 0, 0),
                RiskClass::Low,
            ))
            .expect("v1");
        registry
            .register(descriptor(
                "work.task",
                ContractVersion::new(1, 1, 0),
                RiskClass::Low,
            ))
            .expect("v1.1");
        match registry.resolve("work.task", ContractVersion::new(2, 0, 0)) {
            Resolution::UnsupportedVersion { available } => {
                assert_eq!(
                    available,
                    vec![ContractVersion::new(1, 0, 0), ContractVersion::new(1, 1, 0)]
                );
            }
            other => panic!("expected unsupported version, got {other:?}"),
        }
    }
}
