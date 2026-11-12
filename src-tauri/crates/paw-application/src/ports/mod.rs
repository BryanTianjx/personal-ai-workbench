//! Ports published by the application layer; adapters implement them in
//! paw-infrastructure (ADR-018: ports are defined by the consuming layer).

pub mod agent_adapter;
pub mod agent_repository;
pub mod area_repository;
pub mod attachment_repository;
pub mod audit;
pub mod authorization;
pub mod canonical_plan;
pub mod capability;
pub mod creator_repository;
pub mod dashboard;
pub mod file_store;
pub mod goal_repository;
pub mod knowledge_repository;
pub mod learning_repository;
pub mod operation_service;
pub mod platform_repository;
pub mod plugin_registry;
pub mod search;
pub mod skill_repository;
pub mod task_repository;
pub mod workspace_repository;
