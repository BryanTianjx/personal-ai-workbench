//! SQLite repository implementations of the paw-application ports.
//!
//! These adapters enforce workspace isolation at the repository layer: every
//! query is filtered by the requested workspace_id, cross-workspace
//! references are refused by the composite schema keys, and driver errors
//! are absorbed by private error types so the port contracts only ever see
//! typed, non-leaky errors.

pub mod agent_governance;
pub mod area_repository;
pub mod attachment_repository;
pub mod creator;
pub mod gap_proposals;
pub mod goal_repository;
pub mod knowledge_repository;
pub mod learning_repository;
pub mod retrospectives;
pub mod skills;
pub mod task_repository;
pub mod workspace_repository;
