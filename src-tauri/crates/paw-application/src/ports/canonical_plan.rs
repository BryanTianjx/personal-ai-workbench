//! Canonical invocation plan and digest (E5-02, PAW-INF-006; ADR-029 §4.3).
//!
//! An approval binds to the canonical digest of the immutable invocation
//! plan. Canonicalization must be deterministic across processes and
//! versions so that "the exact action approved" is verifiable:
//!
//! - field order is fixed by the struct (not by user-supplied maps);
//! - parameter maps are sorted by key (serde_json `Map` = `BTreeMap`);
//! - the JSON encoding is compact UTF-8 with no insignificant whitespace;
//! - the digest is SHA-256 over the canonical text, hex-encoded lowercase;
//! - `CANONICALIZATION_VERSION` is recorded with every digest; any change
//!   to the canonical form must bump it and provide migration vectors
//!   (`E5-implementation-specification` §12.4).
//!
//! The plan carries only stable identifiers and material parameters. Secret
//! values never enter a plan (credential references are opaque ids), so the
//! digest itself cannot leak secrets.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Canonical-form version. Bump on any change to the encoding rules.
pub const CANONICALIZATION_VERSION: u32 = 1;

/// A material invocation parameter value. The set is deliberately small:
/// strings, integers, booleans and string lists cover capability inputs
/// without admitting nested untrusted structures.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PlanValue {
    Text(String),
    Int(i64),
    Bool(bool),
    /// Sorted by construction at the map level; list order is material and
    /// preserved (it is part of what was approved).
    List(Vec<String>),
}

impl PlanValue {
    /// Build a string-valued parameter.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
}

/// The immutable invocation plan an approval binds to
/// (`E5-implementation-specification` §5/§6). Every field is material: a
/// change to any of them changes the digest and invalidates a previously
/// granted approval (ADR-029 §4.3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CanonicalPlan {
    pub capability_id: String,
    pub capability_contract_version: String,
    /// Principal the execution will run as (`local-user` or `agent:<id>`).
    pub actor_id: String,
    /// Agent registration executing the capability, when agent-driven.
    pub agent_registration_id: Option<String>,
    /// The Data Workspace the invocation is scoped to (its raw text id).
    pub data_workspace_id: String,
    /// Tool/adapter identity (e.g. `fake`), when a tool mediates the effect.
    pub tool_adapter: String,
    /// Provider/target identity the effect lands on.
    pub target_identity: String,
    /// Material parameters: keys sorted, values limited to `PlanValue`.
    pub parameters: std::collections::BTreeMap<String, PlanValue>,
    /// Declared side-effect set (capability-scoped effect identifiers).
    pub side_effect_set: Vec<String>,
}

impl CanonicalPlan {
    /// Deterministic canonical JSON text (compact, keys sorted, UTF-8).
    /// `canonicalization_version` is part of the encoded object so digests
    /// cannot be compared across incompatible canonical forms.
    pub fn canonical_json(&self) -> String {
        let mut map = serde_json::Map::new();
        map.insert(
            "canonicalization_version".to_string(),
            serde_json::Value::from(CANONICALIZATION_VERSION),
        );
        // serde_json::Value::Object serializes with keys sorted by the
        // BTreeMap backing; nested maps (parameters) sort the same way.
        let value = serde_json::to_value(self).expect("plan serializes");
        let serde_json::Value::Object(inner) = value else {
            unreachable!("CanonicalPlan serializes to an object");
        };
        for (key, val) in inner {
            map.insert(key, val);
        }
        serde_json::to_string(&serde_json::Value::Object(map)).expect("compact text")
    }

    /// SHA-256 digest of the canonical JSON, lowercase hex (64 chars).
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_json().as_bytes());
        let finished = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in finished {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex
    }

    /// Material-parameter digest: SHA-256 over the canonical encoding of
    /// the parameter map alone (bound into the approval record without
    /// copying raw parameter values).
    pub fn material_params_digest(&self) -> String {
        let params = serde_json::to_string(&self.parameters).expect("params serializes");
        let mut hasher = Sha256::new();
        hasher.update(params.as_bytes());
        let finished = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in finished {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn plan() -> CanonicalPlan {
        let mut parameters = BTreeMap::new();
        parameters.insert("title".to_string(), PlanValue::text("Write spec"));
        parameters.insert("priority".to_string(), PlanValue::Int(2));
        parameters.insert("urgent".to_string(), PlanValue::Bool(false));
        CanonicalPlan {
            capability_id: "work_management.task.create".to_string(),
            capability_contract_version: "1.0.0".to_string(),
            actor_id: "local-user".to_string(),
            agent_registration_id: Some("agent-1".to_string()),
            data_workspace_id: "ws-1".to_string(),
            tool_adapter: "fake".to_string(),
            target_identity: "provider:fake".to_string(),
            parameters,
            side_effect_set: vec!["work.task.create".to_string()],
        }
    }

    #[test]
    fn digest_is_deterministic_and_canonical() {
        let a = plan().digest();
        let b = plan().digest();
        assert_eq!(a, b, "same plan must produce the same digest");
        assert_eq!(a.len(), 64);
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn parameter_keys_are_sorted_in_canonical_text() {
        // Map keys are sorted by the canonical encoding (BTreeMap-backed
        // serde_json object), independent of insertion order.
        let mut p = plan();
        p.parameters.insert("aaa".to_string(), PlanValue::Int(1));
        p.parameters.insert("zzz".to_string(), PlanValue::Int(9));
        let text = p.canonical_json();
        let aaa = text.find("\"aaa\"").expect("aaa present");
        let priority = text.find("\"priority\"").expect("priority present");
        let title = text.find("\"title\"").expect("title present");
        let urgent = text.find("\"urgent\"").expect("urgent present");
        let zzz = text.find("\"zzz\"").expect("zzz present");
        assert!(aaa < priority && priority < title && title < urgent && urgent < zzz);
    }

    #[test]
    fn every_field_is_material() {
        // Any change to a material field changes the digest (ADR-029 §4.3;
        // contract tests mutate each material field per the spec §11.4).
        let base = plan();
        let base_digest = base.digest();

        let mutate = |f: &dyn Fn(&mut CanonicalPlan)| {
            let mut p = base.clone();
            f(&mut p);
            assert_ne!(
                p.digest(),
                base_digest,
                "material field change must alter digest"
            );
        };
        mutate(&|p| p.capability_id = "work_management.task.update".to_string());
        mutate(&|p| p.capability_contract_version = "1.1.0".to_string());
        mutate(&|p| p.actor_id = "agent:x".to_string());
        mutate(&|p| p.agent_registration_id = None);
        mutate(&|p| p.data_workspace_id = "ws-2".to_string());
        mutate(&|p| p.tool_adapter = "cli_hermes".to_string());
        mutate(&|p| p.target_identity = "provider:other".to_string());
        mutate(&|p| {
            p.parameters
                .insert("title".to_string(), PlanValue::text("Changed"));
        });
        mutate(&|p| {
            p.parameters.insert("extra".to_string(), PlanValue::Int(1));
        });
        mutate(&|p| p.side_effect_set.push("work.task.delete".to_string()));
    }

    #[test]
    fn canonicalization_version_is_part_of_the_digest() {
        // The canonical text embeds the version so cross-form digests can
        // never collide silently.
        let text = plan().canonical_json();
        assert!(
            text.contains("\"canonicalization_version\":1"),
            "canonical text must embed the version: {text}"
        );
    }

    #[test]
    fn material_params_digest_differs_on_param_change() {
        let mut changed = plan();
        changed
            .parameters
            .insert("title".to_string(), PlanValue::text("Other"));
        assert_ne!(
            plan().material_params_digest(),
            changed.material_params_digest()
        );
    }

    #[test]
    fn list_values_are_serialized() {
        let mut p = plan();
        p.parameters.insert(
            "tags".to_string(),
            PlanValue::List(vec!["a".into(), "b".into()]),
        );
        let text = p.canonical_json();
        assert!(
            text.contains("\"tags\":[\"a\",\"b\"]"),
            "list encoding: {text}"
        );
    }
}
