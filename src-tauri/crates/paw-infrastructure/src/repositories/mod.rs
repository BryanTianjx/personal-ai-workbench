//! Workspace-scoped repository implementations of the paw-application ports.
//!
//! These SQLite adapters enforce workspace isolation at the repository layer:
//! every query is filtered by the requested workspace_id, and cross-workspace
//! mutations are refused at the port boundary.

pub mod workspace_repository;
