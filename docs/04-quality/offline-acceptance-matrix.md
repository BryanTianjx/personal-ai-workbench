# Windows / macOS Offline Acceptance Matrix

**Baseline:** E0.2 Candidate
**Status:** Test specification; not executed
**Verification Status:** Not Verified

## 1. Evidence record
For every test record: test ID, OS version/build, CPU architecture, app version/build hash, installer identity, network-isolation method, clean-profile state, steps, expected and actual result, pass/fail, evidence location, tester, and timestamp. Run each required test independently on Windows and macOS. A build-only or cross-compile result is not acceptance.

## 2. Acceptance cases

| ID | Test | Expected result | Windows | macOS |
|---|---|---|---|---|
| OFF-001 | Install from locally available release installer with network disabled | Installer completes without downloading core runtime dependencies | Required | Required |
| OFF-002 | First launch while offline | App starts; no blocking remote check | Required | Required |
| OFF-003 | Workspace initialize/open/restart | Data persists and reopens | Required | Required |
| OFF-004 | Project and Task CRUD/archive/complete | State and relationships persist | Required | Required |
| OFF-005 | Dashboard | Values match local authoritative queries; remote state is explicit | Required | Required |
| OFF-006 | FTS5 search | Search works offline and cannot reveal another Workspace's records | Required | Required |
| OFF-007 | Attachment create/open/import | Allowed files work; traversal and symlink escape denied | Required | Required |
| OFF-008 | Audit and local history | Available offline and sensitive values redacted | Required | Required |
| OFF-009 | Backup | Consistent database, attachments, manifest and checksums created | Required | Required |
| OFF-010 | Restore in clean profile | Restore succeeds; integrity and attachment checks pass | Required | Required |
| OFF-011 | Migration | Supported upgrade and interrupted migration recover safely | Required | Required |
| OFF-012 | OS credential storage | Secret stored/retrieved through native credential facility; absent from DB/logs | Required | Required |
| OFF-013 | Remote integrations disconnected | Clear Unavailable/Unauthorized/Unknown state; local workflows unaffected | Required | Required |
| OFF-014 | Reconnect | Status refreshes; no duplicate records or remote side effects | Required | Required |
| OFF-015 | Network exposure | No public listener or unapproved outbound transmission | Required | Required |
| OFF-016 | Upgrade/uninstall | User data handling is explicit; upgrade preserves data; uninstall behavior documented | Required | Required |
| OFF-017 | Platform trust | Installer/signature and OS trust behavior validated | Windows signing/SmartScreen policy | macOS signing, notarization, Gatekeeper, first launch |

## 3. Platform-specific checks

- Windows: supported Windows release, native installer, known-folder paths, Credential Manager/DPAPI integration, drive and UNC path handling, symlink/reparse-point behavior, upgrade and uninstall.
- macOS: actual macOS environment, supported architecture(s), signed/notarized package, Gatekeeper and first-launch prompts, Application Support/Logs/Caches paths, Keychain, sandbox/permissions, symlink behavior, upgrade and uninstall.

## 4. Release gate
Alpha 0.1 cannot pass if any P0 case fails on either platform, if data is lost/corrupted, if cross-workspace access succeeds, if remote actions falsely report success, or if an unresolved P0 security defect remains. Each platform must have its own completed evidence set.
