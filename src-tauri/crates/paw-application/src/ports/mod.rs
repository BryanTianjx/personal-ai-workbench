//! Ports published by the application layer; adapters implement them in
//! paw-infrastructure (ADR-018: ports are defined by the consuming layer).

pub mod area_repository;
pub mod audit;
pub mod authorization;
pub mod goal_repository;
pub mod task_repository;
pub mod workspace_repository;
