//! paw-domain: business domain entities, value objects, invariants, and
//! domain events for the Personal AI Workbench.
//!
//! Dependency contract (ADR-018, `E1-rust-workspace-layout.md` §2): this
//! crate depends on the Rust standard library plus narrowly selected pure
//! utility crates only (currently `serde` and `uuid`, enforced by the
//! architecture allowlist test). It must not depend on Tauri, SQL drivers,
//! OS APIs, network clients, provider SDKs, or any other PAW crate.

pub mod ai_collaboration;
pub mod creator;
pub mod knowledge;
pub mod learning;
pub mod operation_machine;
pub mod skills;
pub mod work_management;

pub use operation_machine::OperationStatus;
