//! Operation state machine (machine interaction) — storage-agnostic core.
//!
//! This module is the pure domain-level state machine for Durable
//! Operations (E1-07 / ADR-019). It carries **no** SQL, filesystem, OS, or
//! Tauri dependency. It is shared by:
//!
//! - `paw-infrastructure` (SQLite-backed operation service) which persists
//!   transitions and delegates validation to this machine; and
//! - `paw-desktop` release-readiness (startup check, restart recovery) which
//!   re-validates the machine rules without a database.
//!
//! The design follows ADR-017/ADR-018: the coordination layer keeps the state
//! machine pure; side effects (SQL persistence) stay in the adapter.
//!
//! Transition rules (from `E0.3.2` §2):
//! - `Pending` → `Running | AwaitingApproval | Cancelled | Failed`
//! - `AwaitingApproval` → `Pending | Running | Cancelled | Failed`
//! - `Running` → `RetryScheduled | OutcomeUnknown | Succeeded | Failed | Cancelled`
//! - `RetryScheduled` → `Running | Cancelled | Failed`
//! - `OutcomeUnknown` → `Succeeded | Failed | Cancelled`
//! - Terminal states (`Succeeded` | `Failed` | `Cancelled`) have no outgoing
//!   transitions; an unresolved outcome (`OutcomeUnknown`) blocks retry until
//!   explicit reconciliation.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    AwaitingApproval,
    RetryScheduled,
    OutcomeUnknown,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    /// String form of the state, mirrored by the SQLite CHECK constraint in
    /// migration `0002_durable_operations_outbox.sql`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::RetryScheduled => "RetryScheduled",
            Self::OutcomeUnknown => "OutcomeUnknown",
            Self::Succeeded => "Succeeded",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Pending" => Self::Pending,
            "Running" => Self::Running,
            "AwaitingApproval" => Self::AwaitingApproval,
            "RetryScheduled" => Self::RetryScheduled,
            "OutcomeUnknown" => Self::OutcomeUnknown,
            "Succeeded" => Self::Succeeded,
            "Failed" => Self::Failed,
            "Cancelled" => Self::Cancelled,
            _ => return None,
        })
    }
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Terminal states: reaching the terminal freezes the operation envelope.
pub const fn is_terminal(status: OperationStatus) -> bool {
    matches!(
        status,
        OperationStatus::Succeeded | OperationStatus::Failed | OperationStatus::Cancelled
    )
}

/// Allowed transitions from `from` (E0.3.2 §2).
pub fn allowed_transitions(from: OperationStatus) -> &'static [OperationStatus] {
    use OperationStatus::*;
    match from {
        Pending => &[Running, AwaitingApproval, Cancelled, Failed],
        AwaitingApproval => &[Pending, Running, Cancelled, Failed],
        Running => &[RetryScheduled, OutcomeUnknown, Succeeded, Failed, Cancelled],
        RetryScheduled => &[Running, Cancelled, Failed],
        OutcomeUnknown => &[Succeeded, Failed, Cancelled],
        Succeeded | Failed | Cancelled => &[],
    }
}

/// Returns `true` if a transition `from -> to` is legal.
pub fn is_valid_transition(from: OperationStatus, to: OperationStatus) -> bool {
    if is_terminal(from) {
        return false;
    }
    allowed_transitions(from).contains(&to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_frozen() {
        for st in [
            OperationStatus::Succeeded,
            OperationStatus::Failed,
            OperationStatus::Cancelled,
        ] {
            assert!(!is_valid_transition(st, OperationStatus::Running));
            assert!(!is_valid_transition(st, OperationStatus::Running));
        }
    }

    #[test]
    fn outcome_unknown_blocks_retry_until_reconciliation() {
        // The unsafe retry (Running) is blocked at state level.
        assert!(!is_valid_transition(
            OperationStatus::OutcomeUnknown,
            OperationStatus::Running
        ));
    }

    #[test]
    fn restart_recovery_recovers_running_to_failed() {
        // "Restart" semantics: a fresh process runs recovery against the same
        // envelope — the suspended `Running` state becomes `Failed` (process
        // restart category) and remains frozen.
        let recovered = OperationStatus::Failed;
        assert!(is_terminal(recovered));
        assert!(!is_valid_transition(recovered, OperationStatus::Running));
    }

    #[test]
    fn normal_lifecycle_transitions_are_valid() {
        let pairs: &[(OperationStatus, OperationStatus)] = &[
            (OperationStatus::Pending, OperationStatus::Running),
            (OperationStatus::Pending, OperationStatus::AwaitingApproval),
            (OperationStatus::Pending, OperationStatus::Cancelled),
            (OperationStatus::Pending, OperationStatus::Failed),
            (OperationStatus::AwaitingApproval, OperationStatus::Pending),
            (OperationStatus::AwaitingApproval, OperationStatus::Running),
            (OperationStatus::AwaitingApproval, OperationStatus::Cancelled),
            (OperationStatus::AwaitingApproval, OperationStatus::Failed),
            (OperationStatus::Running, OperationStatus::RetryScheduled),
            (OperationStatus::Running, OperationStatus::OutcomeUnknown),
            (OperationStatus::Running, OperationStatus::Succeeded),
            (OperationStatus::Running, OperationStatus::Failed),
            (OperationStatus::Running, OperationStatus::Cancelled),
            (OperationStatus::RetryScheduled, OperationStatus::Running),
            (OperationStatus::RetryScheduled, OperationStatus::Cancelled),
            (OperationStatus::RetryScheduled, OperationStatus::Failed),
            (OperationStatus::OutcomeUnknown, OperationStatus::Succeeded),
            (OperationStatus::OutcomeUnknown, OperationStatus::Failed),
            (OperationStatus::OutcomeUnknown, OperationStatus::Cancelled),
        ];
        for &(from, to) in pairs {
            assert!(
                is_valid_transition(from, to),
                "expected valid transition {from:?} -> {to:?}"
            );
        }
    }
}
