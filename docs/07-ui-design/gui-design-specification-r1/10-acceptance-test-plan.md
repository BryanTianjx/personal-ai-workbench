# 10 — GUI Acceptance Test Plan

**Status:** Proposed test specification; not executed  
**Delivery platform:** Windows Alpha 0.1

## 1. Evidence requirements

Each test record includes:
- test ID;
- OS/build and screen resolution;
- application version/build hash;
- workspace fixture;
- network-isolation status;
- steps and expected result;
- actual result and pass/fail;
- screenshot/video/log evidence where appropriate;
- tester and timestamp.

## 2. Visual and layout tests

| ID | Test | Expected |
|---|---|---|
| GUI-VIS-001 | Dashboard at 1440×900 | No clipping; hierarchy and spacing consistent |
| GUI-VIS-002 | Project at 1280×720 | Main actions remain reachable |
| GUI-VIS-003 | Task Kanban at compact width | Board scrolls within content region; shell remains stable |
| GUI-VIS-004 | Sidebar collapse/expand | State and focus are preserved |
| GUI-VIS-005 | Long Chinese/English labels | No destructive overflow or overlap |
| GUI-VIS-006 | Empty and partial dashboard | Clear, distinct states |

## 3. Navigation and interaction tests

| ID | Test | Expected |
|---|---|---|
| GUI-NAV-001 | Navigate all enabled Alpha 0.1 entries | Correct page and selected navigation |
| GUI-NAV-002 | Open unknown/deferred deep link | Safe unavailable/not-found state |
| GUI-NAV-003 | Ctrl+K global search | Search opens and focus is correct |
| GUI-NAV-004 | Keyboard-only navigation | All essential actions reachable |
| GUI-NAV-005 | Escape dialog/drawer | Topmost dismissible layer closes; focus restored |

## 4. Business UI tests

| ID | Test | Expected |
|---|---|---|
| GUI-BIZ-001 | Create/edit Goal | Backend-confirmed result displayed |
| GUI-BIZ-002 | Create/edit Area | Backend-confirmed result displayed |
| GUI-BIZ-003 | Create/edit Project | Relationships and lifecycle preserved |
| GUI-BIZ-004 | Create Task | Optional project relationship handled correctly |
| GUI-BIZ-005 | Invalid Task transition | Prior state restored; error explained |
| GUI-BIZ-006 | Search records | Workspace-scoped results only |
| GUI-BIZ-007 | Add/remove attachment | Backend and file-system outcome reconciled |

## 5. Reliability and offline tests

| ID | Test | Expected |
|---|---|---|
| GUI-REL-001 | IPC timeout during read | Feature-level error; shell remains usable |
| GUI-REL-002 | IPC timeout during non-idempotent write | No blind retry; reconcile outcome |
| GUI-REL-003 | Local runtime unavailable | Clear blocking/recoverable state |
| GUI-REL-004 | Network disabled | Core local workflows remain usable |
| GUI-REL-005 | Remote integration unavailable | Local workflow unaffected |
| GUI-REL-006 | Workspace switch | No stale cross-workspace data shown |
| GUI-REL-007 | Concurrent update conflict | Draft preserved; conflict surfaced |

## 6. Security and authorization tests

| ID | Test | Expected |
|---|---|---|
| GUI-SEC-001 | Tamper workspace ID in UI | Backend denies out-of-scope access |
| GUI-SEC-002 | Navigate to deferred Agent route | No execution capability exposed |
| GUI-SEC-003 | Attempt unauthorized mutation | Backend denial shown without optimistic success |
| GUI-SEC-004 | Attachment traversal/symlink escape | Backend rejects |
| GUI-SEC-005 | Inspect frontend logs/errors | No secrets or sensitive payloads leaked |

## 7. Accessibility

- Visible focus indicator.
- Accessible names for icon-only controls.
- Labels associated with form fields.
- Contrast checked against WCAG 2.1 AA.
- Keyboard alternative for drag/drop.
- Error announcement and focus management.

## 8. Release gate

No GUI release sign-off until all P0 security/data integrity tests pass, all P1 Alpha 0.1 workflows pass, and evidence is attached. macOS testing remains deferred to its separate readiness milestone under ADR-021.
