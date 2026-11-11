//! Ports published by the application layer; adapters implement them in
//! paw-infrastructure (ADR-018: ports are defined by the consuming layer).

pub mod area_repository;
pub mod attachment_repository;
pub mod audit;
pub mod authorization;
pub mod canonical_plan;
pub mod capability;
pub mod dashboard;
pub mod file_store;
pub mod goal_repository;
pub mod knowledge_repository;
pub mod learning_repository;
pub mod search;
pub mod task_repository;
pub mod workspace_repository;
