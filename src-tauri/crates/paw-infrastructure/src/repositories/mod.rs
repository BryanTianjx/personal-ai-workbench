//! SQLite repository implementations of the paw-application ports.
//!
//! These adapters enforce workspace isolation at the repository layer: every
//! query is filtered by the requested workspace_id, cross-workspace
//! references are refused by the composite schema keys, and driver errors
//! are absorbed by private error types so the port contracts only ever see
//! typed, non-leaky errors.

pub mod area_repository;
pub mod goal_repository;
pub mod task_repository;
pub mod workspace_repository;
