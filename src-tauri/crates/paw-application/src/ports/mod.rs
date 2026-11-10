//! Ports published by the application layer; adapters implement them in
//! paw-infrastructure (ADR-018: ports are defined by the consuming layer).

pub mod audit;
pub mod authorization;
pub mod workspace_repository;
