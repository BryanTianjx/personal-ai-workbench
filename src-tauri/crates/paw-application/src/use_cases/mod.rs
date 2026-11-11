//! Application use cases: named cross-domain operations published to the
//! desktop boundary (ADR-017: coordination stays inside paw-application).
//!
//! Every use case takes an explicit repository port and workspace context;
//! none of them may touch infrastructure or provider types.

pub mod agent_invocation;
pub mod work_management;
