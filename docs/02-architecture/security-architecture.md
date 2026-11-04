# Security Architecture

**Status:** Proposed — E0.2

## 1. Objectives and threat model

Protect local user data, credentials, workspace boundaries, and user authority over Agent actions. Consider compromised Agent output, prompt injection in imported content, untrusted project files, path traversal, symlink attacks, malicious adapters, credential leakage, accidental publication, database corruption, and unauthorized local/network access. An unlocked compromised OS account remains a residual risk.

## 2. Permission model

Use Read, Write, Execute, and Admin permissions scoped by workspace, project, directory roots, tool, and operation. Deny by default when scope is absent or ambiguous. Provider permissions are additional defense, not a substitute.

Require human approval for shell execution, destructive file/database changes, external publication, credential use, privilege escalation, and broad network access. Approval summarizes exact action, target, scope, and data transfer. Material changes invalidate approval.

## 3. Filesystem controls

Canonicalize paths before access; enforce allowlisted roots and workspace ownership; reject traversal, unexpected absolute paths, and symlink escapes; apply safe filename and import size/type limits; use atomic writes; test Windows drive/UNC paths and macOS symlink/package edge cases.

## 4. Credential handling

Use Windows Credential Manager/DPAPI-backed facilities and macOS Keychain through maintained platform integrations. SQLite stores only credential references and non-secret metadata. Never put tokens in source, observable command-line arguments, logs, crash reports, or exported diagnostics. Redact environment and process output before persistence.

## 5. Network and integration

No public listener by default. Remote connections are opt-in, endpoints visible, TLS validation enabled, and credentials scoped. Do not auto-discover or transmit workspace data to arbitrary endpoints. Imported knowledge and Agent outputs are untrusted data, not instructions to the policy layer.

## 6. Audit and privacy

Record actor, operation, target, decision, approval reference, outcome, and correlation ID. Minimize payload capture; redact secrets/sensitive content. Define retention and export/delete behavior before production. Ordinary module APIs cannot alter audit records.

## 7. Supply chain

Pin and review dependencies, generate build provenance where practical, sign/notarize releases according to platform requirements, and publish checksums. Adapter/plugin installation requires explicit trust and capability review. Updates must not silently expand permissions.

## 8. Verification

Test path traversal, symlink escape, cross-workspace access, permission bypass, stale approval, prompt-injection resistance at action boundaries, secret redaction, malicious imports, and backup confidentiality/integrity.
