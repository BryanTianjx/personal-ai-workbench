//! The in-process fake agent adapter (E5-03).
//!
//! E5's conformance surface: a fully scripted, provider-free adapter that
//! exercises the governed pipeline without external effects. Real
//! transports (Hermes/Codex/API) arrive only after ADR-022 acceptance plus
//! versioned compatibility tests (`E5-implementation-specification` §7).
//!
//! The fake is deliberately honest about uncertainty: scripted
//! `Indeterminate` submissions and `Unknown` statuses flow through the
//! pipeline as unresolved outcomes, exercising the ADR-019 reconciliation
//! rules instead of pretending success.

use std::collections::HashMap;
use std::sync::Mutex;

use paw_application::ports::agent_adapter::{
    AdapterCapabilities, AdapterError, AdapterExecutionStatus, AdapterHealth, AdapterResult,
    AdapterSubmitOutcome, AdapterSubmitRequest, AgentAdapter,
};

/// A scripted step for one execution id. Unscripted ids follow the happy
/// path (accepted → running → completed).
#[derive(Debug, Clone)]
pub enum FakeStep {
    /// submit() returns Accepted with the given external id.
    Accepted(String),
    /// submit() returns Rejected with a reason.
    Rejected(String),
    /// submit() returns Indeterminate — the provider could not confirm
    /// acceptance (the pipeline must treat this as unresolved).
    Indeterminate(String),
    /// status()/result() report this normalized status.
    Status(AdapterExecutionStatus),
    /// status()/cancel() report a transport failure.
    Transport(String),
}

/// The default, unscripted external id for an execution.
fn default_external_id(execution_id: &str) -> String {
    format!("fake-{execution_id}")
}

pub struct FakeAgentAdapter {
    /// Scripted steps per execution id, consumed in order.
    scripts: Mutex<HashMap<String, Vec<FakeStep>>>,
}

impl FakeAgentAdapter {
    pub fn new() -> Self {
        Self {
            scripts: Mutex::new(HashMap::new()),
        }
    }

    /// Queue scripted steps for an execution id (FIFO). Steps with no
    /// matching call are simply unused.
    pub fn script(&self, execution_id: &str, steps: Vec<FakeStep>) {
        self.scripts
            .lock()
            .expect("fake adapter scripts lock")
            .insert(execution_id.to_string(), steps);
    }

    fn next_step(&self, execution_id: &str) -> Option<FakeStep> {
        let mut scripts = self.scripts.lock().expect("fake adapter scripts lock");
        scripts.get_mut(execution_id).and_then(|steps| {
            if steps.is_empty() {
                None
            } else {
                Some(steps.remove(0))
            }
        })
    }

    /// The normalized status an unscripted execution reports (completed).
    fn default_status() -> AdapterExecutionStatus {
        AdapterExecutionStatus::Completed
    }
}

impl Default for FakeAgentAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for FakeAgentAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            transport: "fake".to_string(),
            supports_cancellation: true,
            supports_streaming: false,
            supports_idempotency_key: true,
        }
    }

    fn health(&self) -> AdapterHealth {
        AdapterHealth::Available
    }

    fn submit(&self, request: &AdapterSubmitRequest) -> Result<AdapterSubmitOutcome, AdapterError> {
        match self.next_step(&request.execution_id) {
            Some(FakeStep::Accepted(external_id)) => {
                Ok(AdapterSubmitOutcome::Accepted { external_id })
            }
            Some(FakeStep::Rejected(reason)) => Ok(AdapterSubmitOutcome::Rejected { reason }),
            Some(FakeStep::Indeterminate(reason)) => {
                Ok(AdapterSubmitOutcome::Indeterminate { reason })
            }
            Some(FakeStep::Transport(reason)) => Err(AdapterError::Transport(reason)),
            Some(FakeStep::Status(status)) => {
                // A status step consumed by submit: treat as accepted then
                // remember the status for later status()/result() calls.
                self.script(&request.execution_id, vec![FakeStep::Status(status)]);
                Ok(AdapterSubmitOutcome::Accepted {
                    external_id: default_external_id(&request.execution_id),
                })
            }
            None => Ok(AdapterSubmitOutcome::Accepted {
                external_id: default_external_id(&request.execution_id),
            }),
        }
    }

    fn status(&self, external_id: &str) -> Result<AdapterExecutionStatus, AdapterError> {
        match self.next_step(external_id) {
            Some(FakeStep::Status(status)) => Ok(status),
            Some(FakeStep::Transport(reason)) => Err(AdapterError::Transport(reason)),
            Some(FakeStep::Accepted(external))
            | Some(FakeStep::Rejected(external))
            | Some(FakeStep::Indeterminate(external)) => {
                // A submit-shaped step read as a status probe: still report
                // the default status and preserve the step's external id as
                // a no-op.
                let _ = external;
                Ok(Self::default_status())
            }
            None => Ok(Self::default_status()),
        }
    }

    fn cancel(&self, external_id: &str) -> Result<AdapterExecutionStatus, AdapterError> {
        match self.next_step(external_id) {
            Some(FakeStep::Status(status)) => Ok(status),
            Some(FakeStep::Transport(reason)) => Err(AdapterError::Transport(reason)),
            _ => Ok(AdapterExecutionStatus::Cancelled),
        }
    }

    fn result(&self, external_id: &str) -> Result<AdapterResult, AdapterError> {
        let status = self.status(external_id)?;
        Ok(AdapterResult {
            status,
            // Redaction-safe placeholder: real providers redact through the
            // pipeline's output validation (spec §8).
            summary: "fake adapter result".to_string(),
            artifact_references: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscripted_submit_follows_happy_path() {
        let adapter = FakeAgentAdapter::new();
        let request = AdapterSubmitRequest {
            execution_id: "exec-1".to_string(),
            plan_digest: "a".repeat(64),
            correlation_id: "corr-1".to_string(),
            capability_id: "work_management.task.create".to_string(),
            parameters: Default::default(),
        };
        let outcome = adapter.submit(&request).expect("accepted");
        assert!(matches!(outcome, AdapterSubmitOutcome::Accepted { .. }));
        let external = match outcome {
            AdapterSubmitOutcome::Accepted { external_id } => external_id,
            _ => unreachable!(),
        };
        assert!(matches!(
            adapter.status(&external).expect("status"),
            AdapterExecutionStatus::Completed
        ));
    }

    #[test]
    fn scripted_indeterminate_submit_flows_through() {
        let adapter = FakeAgentAdapter::new();
        adapter.script(
            "exec-2",
            vec![FakeStep::Indeterminate("disconnect after send".to_string())],
        );
        let request = AdapterSubmitRequest {
            execution_id: "exec-2".to_string(),
            plan_digest: "a".repeat(64),
            correlation_id: "corr-2".to_string(),
            capability_id: "work.task".to_string(),
            parameters: Default::default(),
        };
        let outcome = adapter
            .submit(&request)
            .expect("indeterminate is not an error");
        assert!(matches!(
            outcome,
            AdapterSubmitOutcome::Indeterminate { .. }
        ));
    }

    #[test]
    fn scripted_transport_failure_and_cancellation() {
        let adapter = FakeAgentAdapter::new();
        adapter.script(
            "external-3",
            vec![FakeStep::Transport("socket closed".to_string())],
        );
        assert!(adapter.status("external-3").is_err());
        adapter.script(
            "external-4",
            vec![FakeStep::Status(AdapterExecutionStatus::Cancelled)],
        );
        assert_eq!(
            adapter.cancel("external-4").expect("cancelled"),
            AdapterExecutionStatus::Cancelled
        );
    }

    #[test]
    fn capabilities_declare_fake_transport() {
        let adapter = FakeAgentAdapter::new();
        let capabilities = adapter.capabilities();
        assert_eq!(capabilities.transport, "fake");
        assert!(capabilities.supports_cancellation);
        assert!(capabilities.supports_idempotency_key);
    }
}
