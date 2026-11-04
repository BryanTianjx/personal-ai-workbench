# Security Architecture

**Architecture Baseline:** E0.2 Candidate
**Decision Status:** Accepted security principles; detailed policy in agent-authorization-model.md
**Implementation Status:** Not Implemented
**Verification Status:** Not Verified

## 1. Objectives and threat model
Protect local user data, credentials, workspace boundaries, and user authority over Agent actions. Consider compromised Agent output, prompt injection in imported content, untrusted project files, path traversal, symlink attacks, malicious adapters, credential leakage, accidental publication, database corruption, and unauthorized local/network access. An unlocked compromised OS account remains a residual risk.

## 2. Authorization
Use Read, Write, Execute, and Admin permissions scoped by workspace, project, directory roots, tool, and operation. Deny by default when scope is absent, ambiguous, or cannot be proven. The backend authorization service is authoritative; UI visibility and provider permissions are not security controls.

Require human approval for shell execution, destructive file/database changes, external publication, credential use, privilege escalation, and broad network access. Approval summarizes exact action, target, scope, data transfer, plan digest, and expiry. Material changes invalidate approval. Agent cannot grant, elevate, or delegate its own authority.

## 3. Filesystem controls
Canonicalize paths before access; enforce allowlisted roots and workspace ownership; reject traversal, unexpected absolute paths, and symlink escapes; validate after resolving links and immediately before opening where platform APIs permit. Apply safe filename and import size/type limits; use atomic writes; test Windows drive/UNC paths and macOS symlink/package edge cases. File dialogs do not replace backend validation.

## 4. Credential handling
Use Windows Credential Manager/DPAPI-backed facilities and macOS Keychain through maintained platform integrations. SQLite stores only credential references and non-secret metadata. Never put tokens in source, observable command-line arguments, logs, crash reports, or exported diagnostics. Redact environment and process output before persistence. Agents receive mediated operations, not unrestricted credential-store access.

## 5. Network and integration
No public listener by default. Remote connections are opt-in, endpoints visible, TLS validation enabled, and credentials scoped. Do not auto-discover or transmit workspace data to arbitrary endpoints. Imported knowledge and Agent outputs are untrusted data, not instructions to the policy layer.

## 6. Audit and privacy
Record actor, operation, target, policy decision, approval reference, plan digest, outcome, timestamp, and correlation ID. Minimize payload capture; redact secrets/sensitive content. Ordinary module APIs cannot alter or delete audit records. Define retention and export/delete behavior before production.

## 7. Supply chain
Pin and review dependencies, generate build provenance where practical, sign/notarize releases according to platform requirements, and publish checksums. Adapter/plugin installation requires explicit trust and capability review. Updates must not silently expand permissions.

## 8. Verification
Test path traversal, symlink escape, cross-workspace access (including FTS5 and background jobs), permission bypass, stale approval, plan tampering, prompt-injection resistance at action boundaries, secret redaction, malicious imports, and backup confidentiality/integrity.
