# Requirements-to-Architecture Traceability

**Architecture Baseline:** E0.2 Candidate
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

This matrix links the product requirements baseline to architecture ownership and acceptance evidence. Requirement status remains governed by `docs/01-product/requirements-baseline.md`; this document does not imply implementation.

| Requirement | PRD / baseline | Architecture owner | Implementation boundary | Acceptance evidence |
|---|---|---|---|---|
| PAW-F-001 | PRD 3.2 | Workspace, Data | Workspace use case + SQLite transaction | Fresh-profile initialization and restart |
| PAW-F-002 | PRD 3.2 | Project | Project application service | CRUD, archive/restore and relationship tests |
| PAW-F-003 | PRD 3.2 | Task | Task application service | CRUD, completion, optional project relation and restart |
| PAW-F-004 | PRD 3.1 | Dashboard, Application | Composed read model via module queries | Compare displayed values with authoritative queries |
| PAW-F-005 | Requirements §2 | Search, Data | FTS5 projection and authorized result lookup | Search relevance, scope filtering and reopen |
| PAW-F-006 | PRD 3.8 | Filesystem, Security | Managed attachment service | Scope, traversal, symlink and persistence tests |
| PAW-F-007 | PRD 3.8 | Data, System | Consistent DB + attachment snapshot | Restore in clean profile and verify checksums |
| PAW-F-008 | PRD 3.8; Requirements §2 | System & Security | Audit writer owned by system boundary | Defined critical-event catalog, redaction, append-only behavior and integrity tests |
| PAW-N-001 | PRD 4 | Platform | Windows native packaging | Install, launch, upgrade, uninstall on Windows |
| PAW-N-002 | PRD 4 | Platform | macOS native packaging | Install, launch, signing, notarization, Gatekeeper |
| PAW-N-003 | PRD 4 | Offline, Platform | No-network core runtime | Full offline matrix independently on both platforms |
| PAW-N-004 | PRD 4 | Data | Versioned SQLite migrations | Upgrade, interruption recovery and data integrity |
| PAW-S-001 | PRD 3.8 | Security, Platform | No public listener by default | Port/process/network inspection |
| PAW-S-002 | PRD 5 | Security | Credential references + redaction | Logs, crash reports and diagnostics scan |
| PAW-S-003 | PRD 3.8 | Security, Filesystem | Canonical path and workspace scope enforcement | Traversal, UNC/drive, symlink and cross-scope tests |
| PAW-S-004 | Requirements §3; Agent Authorization Model | Agent Authorization | Central policy check before every protected side effect; approval cannot widen grants | Alpha 0.2 gate: default-deny, scope enforcement, approval-plan binding, audit and negative tests pass before Agent execution is enabled |
| PAW-AI-001 | PRD 3.3 | AI/Agent | Provider-neutral Adapter contract | Fake-provider contract suite |
| PAW-AI-002 | PRD 3.3 | AI/Agent | Persisted Agent Execution lifecycle | Timeout, cancellation, unknown outcome and result tests |
| PAW-K-001 | PRD 3.4 | Knowledge | External reference model | Reference persistence and unavailable-source behavior |
| PAW-K-002 | Requirements §3 | Knowledge Adapter | Hindsight/Lingshu adapter boundary | Versioned connector contract and outage tests |
| PAW-C-001 | PRD 3.6 | Creator | Topic and draft use cases | Topic/draft CRUD and persistence |
| PAW-C-002 | PRD 3.6 | Creator | Version/source/review records | Version history, reviewer and evidence-state tests |
| PAW-C-003 | PRD 3.6 | Creator Adapter | Capability-verified publishing path | Official API review and real-account verification; manual export fallback |
| PAW-E-001 | Requirements §3 | Engineering | Repository reference adapter | Repository identity and availability tests |
| PAW-AU-001 | PRD 3.7 | Automation | Workflow definition/run records | Trigger authorization and run lifecycle tests |

## Traceability rules

1. Every requirement must map to an architecture owner, implementation boundary, and acceptance evidence before implementation starts.
2. Tests must assert observable behavior and preserve evidence (logs/screenshots/reports) without secrets.
3. Use statuses proposed, approved, implemented, verified, deferred, rejected from the requirements baseline. Only a passing test with retained evidence may set verified.
4. Any requirement with no owner, no test, or conflicting version scope is a review blocker.
