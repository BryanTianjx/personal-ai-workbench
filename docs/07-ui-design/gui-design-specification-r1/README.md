# GUI Design Specification R1

**Product:** Personal AI Workbench  
**Baseline:** Alpha 0.1 (Windows delivery)  
**Status:** Design baseline candidate; prototype review required  
**Version:** R1.0  
**Branch:** `docs/gui-design-spec-r1`

## 1. Purpose

This package defines the frontend experience, design system, navigation, core-page behavior, component architecture, IPC integration boundary, interaction states, prototypes, and acceptance criteria for Personal AI Workbench.

It is an incremental design baseline for the existing repository. It does not authorize changes to domain models, Rust commands, database schema, release scope, or security policy.

## 2. Normative source hierarchy

When requirements conflict, apply this order:

1. Accepted ADRs and explicitly frozen product/architecture baselines on the target branch.
2. Product requirements and feature/version matrix.
3. Existing Application/IPC contracts and domain lifecycle rules.
4. This GUI Design Specification R1.
5. Interactive prototypes and visual examples.

A prototype is illustrative. It does not override a backend contract or prove that a feature is implemented.

## 3. Alpha 0.1 scope

The delivery target is Windows. macOS remains a later readiness milestone under ADR-021; do not claim macOS acceptance in Alpha 0.1.

The UI baseline covers the currently approved Work Management capabilities: Workspace, Dashboard, Goal, Area, Project, Task, Inbox/capture where supported, Search, Attachments, and Settings/backup surfaces only where corresponding application contracts exist.

Learning & Growth, advanced Knowledge management, Agent execution, Workflow authoring, and the fully executable AI Assistant are not silently promoted into Alpha 0.1. They may appear in architecture notes or future-facing prototypes only when visibly labelled as deferred/proposed.

## 4. Package contents

| File | Responsibility |
|---|---|
| 01-design-system.md | Tokens, typography, spacing, elevation, component visual rules |
| 02-application-shell.md | Window frame, shell layout, responsive desktop behavior |
| 03-navigation-and-routing.md | Navigation hierarchy, route policy, feature availability |
| 04-core-page-specification.md | Dashboard, Project, Task/Kanban, Assistant prototype requirements |
| 05-ai-assistant-ui.md | Future assistant UI, action preview and authorization boundary |
| 06-component-architecture.md | React feature boundaries and component contracts |
| 07-ipc-integration-contract.md | Frontend-to-Tauri IPC mapping and safety rules |
| 08-interaction-state-specification.md | Loading, empty, error, offline, conflict and confirmation states |
| 09-design-review-report.md | Review findings, decisions, open issues and exit gates |
| 10-acceptance-test-plan.md | Visual, functional, accessibility and offline acceptance |
| prototypes/ | Runnable React + TypeScript interactive prototypes |

## 5. Status semantics

- **Accepted:** aligned with current frozen baseline and suitable as a design constraint.
- **Proposed:** design recommendation requiring product/engineering ratification.
- **Deferred:** explicitly outside Alpha 0.1.
- **Unverified:** repository evidence or runtime behavior has not yet been validated.

## 6. Non-goals

This package does not implement production UI, modify existing React application code, alter IPC command signatures, change Rust domain behavior, introduce remote dependencies, or authorize Agent operations.

## 7. Review and freeze

R1 becomes a frozen implementation baseline only after:
1. the four prototypes receive visual and interaction review;
2. open P1 findings DR-002, DR-006 and DR-007 are resolved or formally accepted with scope;
3. IPC mappings are checked against actual Rust command DTOs and authorization behavior;
4. acceptance criteria are approved.

Until then, implementation tasks generated from this package must be marked blocked by the GUI baseline approval gate.
