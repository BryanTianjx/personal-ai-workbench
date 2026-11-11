//! Agent adapter port (E5-03; ADR-004, `agent-adapter-architecture.md` §2).
//!
//! The adapter translates the governed request to supported external
//! provider interfaces. It does not authorize, does not own domain
//! semantics, and cannot widen approved scope (§4). E5 ships only the
//! in-process fake adapter; real transports (Hermes/Codex/API) wait for
//! ADR-022 acceptance plus versioned compatibility tests.

/// What an adapter can do (declared as data — limitations are facts the
/// governance layer reads, not decisions it obeys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterCapabilities {
    /// Transport identifier (`fake`, `cli_hermes`, ...).
    pub transport: String,
    pub supports_cancellation: bool,
    pub supports_streaming: bool,
    /// Whether the provider offers an idempotency key on submit.
    pub supports_idempotency_key: bool,
}

/// Health probe result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterHealth {
    Available,
    Degraded { reason: &'static str },
    Unavailable { reason: &'static str },
}

/// Provider-side execution status, normalized (adapter §3: the adapter
/// reports; it never converts an indeterminate result into success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterExecutionStatus {
    /// Accepted and queued/running remotely.
    Running,
    /// Established outcome.
    Completed,
    Failed,
    Cancelled,
    /// The provider reports a timeout; outcome not established.
    TimedOut,
    /// The provider could not establish an outcome (disconnect, unknown
    /// external id) — must surface as unresolved, never inferred.
    Unknown {
        reason: String,
    },
}

/// A submission request produced by the governance pipeline. The adapter
/// treats every field as data; nothing here widens authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSubmitRequest {
    /// Workbench execution id — used as the idempotency key where
    /// supported (adapter §6).
    pub execution_id: String,
    /// Canonical invocation plan digest (traceability; adapter never
    /// re-derives authorization from it).
    pub plan_digest: String,
    /// Correlation id from the Operation envelope.
    pub correlation_id: String,
    /// Opaque capability reference (already resolved and authorized).
    pub capability_id: String,
    /// Material parameters already validated by the pipeline.
    pub parameters: std::collections::BTreeMap<String, String>,
}

/// The result of a submit call (adapter §3: a submit timeout after the
/// request may have reached the provider must map to an indeterminate
/// outcome at the pipeline level).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterSubmitOutcome {
    /// The provider accepted and returned its execution id.
    Accepted { external_id: String },
    /// The provider rejected the submission outright (nothing started).
    Rejected { reason: String },
    /// The submit call failed before the provider could confirm acceptance
    /// OR after the request may have been accepted (disconnect/timeout).
    Indeterminate { reason: String },
}

/// Normalized result (adapter §5). No sensitive transcripts by default;
/// summaries are redaction-safe strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterResult {
    pub status: AdapterExecutionStatus,
    pub summary: String,
    /// Stable, non-secret artifact references (empty in E5).
    pub artifact_references: Vec<String>,
}

/// Adapter errors are transport-level facts; they never carry authorization
/// semantics (a denial would have happened before dispatch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The operation is declared unsupported by this adapter (explicit
    /// not-supported result, adapter §2).
    NotSupported { operation: &'static str },
    /// Transport failure while the request state is known.
    Transport(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSupported { operation } => write!(f, "adapter does not support {operation}"),
            Self::Transport(msg) => write!(f, "adapter transport failure: {msg}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// The adapter contract (adapter §2). All methods are side-effect-explicit:
/// only `submit` and `cancel` reach the provider, and both are called only
/// from inside the governed pipeline after fresh authorization.
pub trait AgentAdapter {
    fn capabilities(&self) -> AdapterCapabilities;
    fn health(&self) -> AdapterHealth;
    fn submit(&self, request: &AdapterSubmitRequest) -> Result<AdapterSubmitOutcome, AdapterError>;
    fn status(&self, external_id: &str) -> Result<AdapterExecutionStatus, AdapterError>;
    fn cancel(&self, external_id: &str) -> Result<AdapterExecutionStatus, AdapterError>;
    fn result(&self, external_id: &str) -> Result<AdapterResult, AdapterError>;
}
