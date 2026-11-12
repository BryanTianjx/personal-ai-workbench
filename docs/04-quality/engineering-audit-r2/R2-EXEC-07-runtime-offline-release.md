# R2-EXEC-07 — Runtime, Offline & Release Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Dependency: R2-EXEC-01 baseline frozen
- Target: supported Windows desktop experience

## Objective

Establish whether the product can be built reproducibly, launched, operated offline, installed, upgraded, uninstalled, and recovered safely.

## Scope

Windows 11, WSL2 development environment, Tauri runtime, WebView2, SQLite, startup/shutdown, offline paths, installers, CI and release gates.

## Audit tasks

### A. Build reproducibility

1. Inspect CI workflows and build scripts.
2. Verify lockfile usage and pinned toolchain.
3. Run configured frontend checks and production build.
4. Run Rust checks and release build.
5. Inspect warnings and nondeterministic build inputs.
6. Inventory generated installers and checksums if available.
7. Record exact commands, versions, SHA, exit codes, durations, and artifacts.

### B. Runtime lifecycle

Use a disposable application profile and DB:
1. Launch and verify first-run initialization.
2. Exercise core local workflows.
3. Shut down normally and restart.
4. Verify synthetic data persistence.
5. Simulate controlled process interruption if safe.
6. Inspect logs, crash handling, lock files, and recovery behavior.
7. Verify the application does not silently fall back to an unintended data directory.

### C. Offline capability matrix

Evaluate separately:
- application startup without network;
- local CRUD and search;
- local attachments/files;
- migrations and recovery;
- AI features;
- Hermes/external Agent integration;
- plugin/catalog operations;
- update checks and telemetry;
- dependency/runtime availability after install.

Use controlled network isolation in a disposable VM or isolated test environment. Do not change system-wide firewall settings. If isolation is unavailable, report NOT_VERIFIED for affected items; local-first architecture is not proof of full offline operation.

### D. Windows packaging and lifecycle

Inspect MSI/NSIS configuration and test generated artifacts in a disposable VM or dedicated test machine:
- install;
- first launch;
- user-data location and permissions;
- upgrade from supported version;
- preservation of existing synthetic data;
- failed upgrade/recovery;
- uninstall behavior;
- residual user data policy.

Never install over the user's existing application or modify their profile. If a disposable VM is unavailable, static installer inspection may proceed but runtime acceptance is BLOCKED.

### E. Release gates

Evaluate Alpha 0.1, Alpha 0.2, Beta 0.5, and V1.0 independently. For every gate, list mandatory criteria, implementation evidence, runtime/test evidence, unresolved findings, and acceptance owner/record. Do not infer gate acceptance from stage labels or successful compilation.

## Required outputs

- `R2-EXEC-07-build-reproducibility-report.md`
- `R2-EXEC-07-runtime-report.md`
- `R2-EXEC-07-offline-report.md`
- `R2-EXEC-07-windows-install-upgrade-report.md`
- `R2-EXEC-07-release-gate-report.md`

## Acceptance criteria

Build results are tied to exact SHA. Runtime tests use a disposable profile. Offline claims are feature-specific. Installer configuration and actual installation behavior are distinguished. Upgrade/data preservation are verified or BLOCKED. No existing installation or global setting is changed.

## Stop conditions

Stop if testing could overwrite an existing installation/profile, require system-wide firewall/config changes, install unapproved dependencies, or access production data.
