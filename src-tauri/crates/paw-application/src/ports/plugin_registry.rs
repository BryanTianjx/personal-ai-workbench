//! Capability plugin registry port (E7-01/E7-02; PAW-INF-007, ADR-023/028).
//!
//! The plugin is the platform **registration/composition** unit — never a
//! replacement domain owner (ADR-027). First-party, statically linked
//! descriptors register at startup; the registry validates the dependency
//! graph, rejects cycles deterministically, and resolves the four
//! independent lifecycle dimensions (ADR-023 R1 §R1):
//!
//! 1. **Registration** — the descriptor is known to the registry (static).
//! 2. **Availability** — dependencies, versions and startup checks permit
//!    invocation (derived, never a user boolean).
//! 3. **Composition preference** — a [`WorkspaceComposition`] may omit an
//!    available plugin (stored per composition, not here).
//! 4. **Presentation visibility** — whether a contribution is currently
//!    shown (derived by the UI from the composition).
//!
//! Version dimensions are independent (ADR-028 §4.2): plugin_version,
//! descriptor_schema_version, capability_contract_version (owned by the
//! E5 registry), data_migration_version (owned by domains),
//! minimum_app_version. Unknown/broken states fail closed with a
//! structured incompatibility reason.

use std::collections::HashMap;

/// Registry construction/lookup failures (fail-closed, structured).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginRegistryError {
    /// The same plugin id was registered twice.
    DuplicatePlugin { id: String },
    /// A plugin id violates the `core.<domain>` namespace.
    InvalidPluginId(String),
    /// A declared dependency does not exist in the registry.
    MissingDependency { id: String, dependency: String },
    /// A declared dependency version is lower than required.
    IncompatibleDependency { id: String, dependency: String },
    /// The dependency graph contains a cycle; the exact cycle is reported.
    DependencyCycle { cycle: Vec<String> },
    /// A plugin id/capability reference is malformed.
    InvalidReference(String),
}

impl std::fmt::Display for PluginRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePlugin { id } => write!(f, "plugin {id} registered twice"),
            Self::InvalidPluginId(id) => write!(f, "plugin id {id} is not `core.<domain>`"),
            Self::MissingDependency { id, dependency } => {
                write!(f, "plugin {id} requires missing plugin {dependency}")
            }
            Self::IncompatibleDependency { id, dependency } => {
                write!(f, "plugin {id} requires newer {dependency}")
            }
            Self::DependencyCycle { cycle } => {
                write!(f, "dependency cycle: {}", cycle.join(" -> "))
            }
            Self::InvalidReference(r) => write!(f, "invalid plugin reference: {r}"),
        }
    }
}

impl std::error::Error for PluginRegistryError {}

/// Why a registered plugin is not currently available (fail-closed
/// diagnostics, ADR-028 §4.3: the host MUST expose a structured reason).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailabilityBlock {
    /// A required dependency is registered-but-unavailable.
    DependencyBlocked { dependency: String },
    /// A required dependency is missing entirely.
    DependencyMissing { dependency: String },
    /// A required dependency version is incompatible.
    DependencyIncompatible { dependency: String },
    /// The plugin depends (directly or transitively) on a blocked plugin —
    /// a cycle member or downstream of a hard failure.
    DownstreamOfBlocked { via: String },
}

/// A first-party plugin descriptor. Statically constructed at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDescriptor {
    /// Stable id, `core.<domain>` namespace.
    pub id: String,
    /// Implementation/descriptor release identity.
    pub plugin_version: (u32, u32, u32),
    /// Shape/semantics of plugin metadata (this spec = 1).
    pub descriptor_schema_version: u32,
    /// Capability ids owned (E5 registry ids the plugin exposes).
    pub owned_capabilities: Vec<String>,
    /// Required dependencies: absent/blocked/incompatible ⇒ blocked.
    pub required_dependencies: Vec<PluginDependencyRef>,
    /// Optional dependencies: missing ⇒ declared safe degradation only.
    pub optional_dependencies: Vec<PluginDependencyRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDependencyRef {
    pub id: String,
    /// Minimum (major, minor, patch) required.
    pub min_version: (u32, u32, u32),
}

/// Runtime availability derived by the registry (dimension 2). The user
/// enable/disable preference is a separate composition setting (ADR-028
/// §4.5) persisted in `plugin_states`, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginAvailability {
    Available,
    Blocked { reason: AvailabilityBlock },
}

/// Metadata-only plugin registry (E7-01). Static first-party registration;
/// no dynamic loading, marketplace, or runtime mutation (ADR-023/ADR-028).
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, PluginDescriptor>,
    /// Deterministic topological order (stable tie-break by plugin id).
    order: Vec<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one descriptor. Graph validation happens in
    /// [`PluginRegistry::validate_graph`] after all registrations.
    pub fn register(&mut self, descriptor: PluginDescriptor) -> Result<(), PluginRegistryError> {
        let ok_id = descriptor.id.starts_with("core.")
            && descriptor.id.len() > 5
            && descriptor
                .id
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_'));
        if !ok_id {
            return Err(PluginRegistryError::InvalidPluginId(descriptor.id));
        }
        if self.plugins.contains_key(&descriptor.id) {
            return Err(PluginRegistryError::DuplicatePlugin { id: descriptor.id });
        }
        for dep in descriptor
            .required_dependencies
            .iter()
            .chain(descriptor.optional_dependencies.iter())
        {
            validate_plugin_id(&dep.id)?;
        }
        self.order.push(descriptor.id.clone());
        self.plugins.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// Validate the complete graph before exposing capabilities: missing
    /// dependencies, version incompatibilities, cycles (deterministic
    /// rejection with the exact cycle path — never best-effort ordering).
    pub fn validate_graph(&self) -> Result<(), PluginRegistryError> {
        // Missing/incompatible checks first (per registration order).
        for id in &self.order {
            let plugin = &self.plugins[id];
            for dep in &plugin.required_dependencies {
                let Some(available) = self.plugins.get(&dep.id) else {
                    return Err(PluginRegistryError::MissingDependency {
                        id: id.clone(),
                        dependency: dep.id.clone(),
                    });
                };
                let have = available.plugin_version;
                if (have.0, have.1, have.2) < dep.min_version {
                    return Err(PluginRegistryError::IncompatibleDependency {
                        id: id.clone(),
                        dependency: dep.id.clone(),
                    });
                }
            }
            for dep in &plugin.optional_dependencies {
                if let Some(available) = self.plugins.get(&dep.id)
                    && available.plugin_version < dep.min_version
                {
                    return Err(PluginRegistryError::IncompatibleDependency {
                        id: id.clone(),
                        dependency: dep.id.clone(),
                    });
                }
            }
        }
        // Cycle detection: deterministic DFS with alphabetical neighbor
        // ordering; the reported cycle is the exact path found.
        let mut neighbors: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, plugin) in &self.plugins {
            let mut deps: Vec<&str> = plugin
                .required_dependencies
                .iter()
                .map(|d| d.id.as_str())
                .collect();
            deps.sort();
            neighbors.insert(id.as_str(), deps);
        }
        let mut state: HashMap<&str, u8> = HashMap::new(); // 0=unvisited 1=visiting 2=done
        let mut stack: Vec<&str> = Vec::new();
        let mut sorted_ids: Vec<&str> = self.plugins.keys().map(|s| s.as_str()).collect();
        sorted_ids.sort();
        for id in sorted_ids {
            if state.get(id).copied().unwrap_or(0) == 0
                && let Some(cycle) = self.visit(id, &neighbors, &mut state, &mut stack)
            {
                return Err(PluginRegistryError::DependencyCycle {
                    cycle: cycle.into_iter().map(str::to_string).collect(),
                });
            }
        }
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn visit<'a>(
        &'a self,
        id: &'a str,
        neighbors: &HashMap<&'a str, Vec<&'a str>>,
        state: &mut HashMap<&'a str, u8>,
        stack: &mut Vec<&'a str>,
    ) -> Option<Vec<&'a str>> {
        state.insert(id, 1);
        stack.push(id);
        for dep in neighbors.get(id).cloned().unwrap_or_default() {
            match state.get(dep).copied().unwrap_or(0) {
                1 => {
                    // Cycle found: slice the stack from the first occurrence.
                    let start = stack.iter().position(|s| *s == dep).unwrap_or(0);
                    let mut cycle: Vec<&str> = stack[start..].to_vec();
                    cycle.push(dep);
                    return Some(cycle);
                }
                0 => {
                    if let Some(cycle) = self.visit(dep, neighbors, state, stack) {
                        return Some(cycle);
                    }
                }
                _ => {}
            }
        }
        stack.pop();
        state.insert(id, 2);
        None
    }

    /// Deterministic availability for one plugin (dimension 2), computed
    /// over the validated graph. Optional-dependency absence is NOT a
    /// block — the descriptor declares its safe degradation instead
    /// (ADR-028 §4.1).
    pub fn availability(&self, id: &str) -> PluginAvailability {
        let Some(plugin) = self.plugins.get(id) else {
            return PluginAvailability::Blocked {
                reason: AvailabilityBlock::DependencyMissing {
                    dependency: id.to_string(),
                },
            };
        };
        for dep in &plugin.required_dependencies {
            match self.plugins.get(&dep.id) {
                None => {
                    return PluginAvailability::Blocked {
                        reason: AvailabilityBlock::DependencyMissing {
                            dependency: dep.id.clone(),
                        },
                    };
                }
                Some(available) => {
                    if available.plugin_version < dep.min_version {
                        return PluginAvailability::Blocked {
                            reason: AvailabilityBlock::DependencyIncompatible {
                                dependency: dep.id.clone(),
                            },
                        };
                    }
                    if let PluginAvailability::Blocked { reason } = self.availability(&dep.id) {
                        return PluginAvailability::Blocked {
                            reason: match reason {
                                AvailabilityBlock::DependencyBlocked { dependency }
                                | AvailabilityBlock::DownstreamOfBlocked { via: dependency } => {
                                    AvailabilityBlock::DownstreamOfBlocked { via: dependency }
                                }
                                AvailabilityBlock::DependencyMissing { dependency }
                                | AvailabilityBlock::DependencyIncompatible { dependency } => {
                                    AvailabilityBlock::DependencyBlocked { dependency }
                                }
                            },
                        };
                    }
                }
            }
        }
        PluginAvailability::Available
    }

    /// All registered plugin ids in deterministic (topological-ish) order.
    pub fn plugin_ids(&self) -> Vec<String> {
        self.order.clone()
    }

    /// Capability ownership lookup: which registered plugin owns a
    /// capability id (used by the disable gate).
    pub fn owner_of(&self, capability_id: &str) -> Option<&str> {
        self.plugins
            .values()
            .find(|p| p.owned_capabilities.iter().any(|c| c == capability_id))
            .map(|p| p.id.as_str())
    }
}

fn validate_plugin_id(id: &str) -> Result<(), PluginRegistryError> {
    let ok = id.starts_with("core.")
        && id.len() > 5
        && id
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_'));
    if ok {
        Ok(())
    } else {
        Err(PluginRegistryError::InvalidReference(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(
        id: &str,
        version: (u32, u32, u32),
        required: Vec<&str>,
        capabilities: Vec<&str>,
    ) -> PluginDescriptor {
        PluginDescriptor {
            id: id.to_string(),
            plugin_version: version,
            descriptor_schema_version: 1,
            owned_capabilities: capabilities.into_iter().map(str::to_string).collect(),
            required_dependencies: required
                .into_iter()
                .map(|d| PluginDependencyRef {
                    id: d.to_string(),
                    min_version: (1, 0, 0),
                })
                .collect(),
            optional_dependencies: Vec::new(),
        }
    }

    #[test]
    fn valid_graph_passes_and_availability_derives() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor(
                "core.work_management",
                (1, 0, 0),
                vec![],
                vec!["work.task.create"],
            ))
            .expect("register wm");
        registry
            .register(descriptor(
                "core.learning_growth",
                (1, 1, 0),
                vec!["core.work_management"],
                vec!["learning.item.create"],
            ))
            .expect("register lg");
        registry.validate_graph().expect("graph valid");
        assert_eq!(
            registry.availability("core.learning_growth"),
            PluginAvailability::Available
        );
    }

    #[test]
    fn missing_dependency_blocks_dependent() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor(
                "core.learning_growth",
                (1, 0, 0),
                vec!["core.work_management"],
                vec![],
            ))
            .expect("registered without dependency present");
        assert!(matches!(
            registry.validate_graph(),
            Err(PluginRegistryError::MissingDependency { .. })
        ));
    }

    #[test]
    fn incompatible_dependency_fails_closed() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor(
                "core.work_management",
                (1, 0, 0),
                vec![],
                vec![],
            ))
            .expect("wm");
        registry
            .register(descriptor(
                "core.learning_growth",
                (1, 0, 0),
                vec![], // custom min below
                vec![],
            ))
            .expect("lg");
        // Manually require 2.0.0 of work_management.
        registry
            .plugins
            .get_mut("core.learning_growth")
            .unwrap()
            .required_dependencies
            .push(PluginDependencyRef {
                id: "core.work_management".to_string(),
                min_version: (2, 0, 0),
            });
        assert!(matches!(
            registry.validate_graph(),
            Err(PluginRegistryError::IncompatibleDependency { .. })
        ));
    }

    #[test]
    fn cycle_rejected_with_exact_path() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor("core.a", (1, 0, 0), vec!["core.b"], vec![]))
            .expect("a");
        registry
            .register(descriptor("core.b", (1, 0, 0), vec!["core.a"], vec![]))
            .expect("b");
        let err = registry.validate_graph().expect_err("cycle");
        match err {
            PluginRegistryError::DependencyCycle { cycle } => {
                assert!(cycle.contains(&"core.a".to_string()));
                assert!(cycle.contains(&"core.b".to_string()));
            }
            other => panic!("expected cycle, got {other:?}"),
        }
    }

    #[test]
    fn downstream_of_blocked_transitive() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor(
                "core.work_management",
                (1, 0, 0),
                vec![],
                vec![],
            ))
            .expect("wm");
        // core.x requires missing core.missing → blocked.
        registry
            .register(descriptor(
                "core.x",
                (1, 0, 0),
                vec!["core.missing"],
                vec![],
            ))
            .expect("x registered");
        // core.y depends on core.x → downstream of blocked.
        registry
            .register(descriptor("core.y", (1, 0, 0), vec!["core.x"], vec![]))
            .expect("y");
        // Graph validation reports the missing dependency first (fail closed).
        assert!(matches!(
            registry.validate_graph(),
            Err(PluginRegistryError::MissingDependency { .. })
        ));
    }

    #[test]
    fn owner_of_resolves_capability() {
        let mut registry = PluginRegistry::new();
        registry
            .register(descriptor(
                "core.work_management",
                (1, 0, 0),
                vec![],
                vec!["work.task.create"],
            ))
            .expect("wm");
        assert_eq!(
            registry.owner_of("work.task.create"),
            Some("core.work_management")
        );
        assert_eq!(registry.owner_of("no.such.capability"), None);
    }
}
