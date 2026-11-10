//! paw-infrastructure: SQLite repositories and migrations, filesystem,
//! credential store, and external provider adapters implementing the ports
//! defined by paw-application.
//!
//! Dependency contract (ADR-018): implements inward-facing ports; it must
//! not depend on paw-desktop and must keep SQL row types private to this
//! crate.

pub mod architecture_checks;
pub mod backup;
pub mod operations;
pub mod outbox;
pub mod repositories;
pub mod sqlite;
