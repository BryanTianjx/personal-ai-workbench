# 05 — AI Assistant UI Specification

**Status:** Deferred capability design; not an Alpha 0.1 execution commitment

## 1. Purpose

Define a future UI pattern for contextual AI assistance without weakening Workbench authorization, data ownership or Agent adapter boundaries.

## 2. Placement

The Assistant is a collapsible context panel or dedicated route. It must not permanently reduce the primary content area. The current Alpha 0.1 build may omit the entry entirely.

## 3. Context model

Context is explicit and inspectable:
- current workspace identifier;
- current page/entity reference;
- user-selected records;
- permitted local context references.

The UI must indicate which context is being shared. It must not silently attach unrelated workspace data or secrets.

## 4. Message and action distinction

Separate:
- user/assistant conversation;
- informational recommendation;
- proposed action plan;
- authorization request;
- execution state/result.

A generated suggestion is not an executed operation. A user click is not itself a capability grant.

## 5. Action preview

An action preview must display:
- exact operation and target;
- proposed changes/diff;
- affected resources and workspace;
- tools/provider and data transfer where relevant;
- risk classification;
- permission status;
- approval expiry and plan digest reference where applicable.

Any material change to target, payload semantics, tool, permission or scope invalidates prior approval.

## 6. Authorization boundary

Backend authorization is authoritative. Default deny, least privilege and explicit resource scope apply. Re-evaluate policy immediately before every protected side effect. UI confirmation cannot expand a grant. High-risk actions require the exact approval semantics defined by agent-authorization-model.md.

The UI must not:
- grant Read/Write/Execute/Admin capabilities;
- widen filesystem roots;
- bypass authorization because a user accepted a suggestion;
- automatically retry an indeterminate non-idempotent action;
- report success before backend/provider confirmation.

## 7. Execution states

Support Created, Validating, AwaitingApproval, Approved, Submitted, Running, Completed, Failed, Cancelled, TimedOut and OutcomeUnknown where the backend lifecycle exposes them. OutcomeUnknown must be visually prominent and must not be represented as Failed or Succeeded.

## 8. Prototype limitations

The bundled assistant prototype is local-only mock UI. It contains no provider adapter, no network call, no credential access and no persistent mutation. Its controls are deliberately disabled or simulate preview-only state.
