//! paw-application: use cases, published ports and DTOs, authorization
//! orchestration, and the logical Coordination Layer.
//!
//! Dependency contract (ADR-018): depends on paw-domain only among PAW
//! crates. It must not depend on paw-infrastructure or paw-desktop, and it
//! must not leak SQL, Tauri, or provider SDK types into its contracts.

pub mod ports;
