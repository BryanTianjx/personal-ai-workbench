# R2-EXEC-01 — Baseline & Environment Audit

- Parent: [Engineering Audit R2.0](./README.md)
- Type: prerequisite / read-only
- Required before: R2-EXEC-02 through R2-EXEC-07
- Suggested owner: audit coordinator

## Objective

Freeze one immutable source baseline and record the local environment so every subsequent result can be reproduced and attributed.

## Scope

Repository identity, branch and commit state, working tree, source tree, Rust workspace, frontend package manager, dependency lockfiles, CI/build scripts, test inventory, migrations, ADRs, E0–E7 reports, GUI R1 documents, local OS/toolchain.

## Procedure

1. Resolve the repository root and verify remote points to `BryanTianjx/personal-ai-workbench`.
2. Record `git remote -v`, `git branch --show-current`, `git rev-parse HEAD`, `git rev-parse origin/main` if origin is available, and `git status --short`.
3. Record the latest 20 commits and relevant branch divergence. Do not fetch if network access is not authorized or unavailable; record the limitation.
4. Compare local main with the package reference SHA `cc54dace9caa0f40be2fcfacba0cd9873d409be3`. Agree one immutable audit SHA. Never reset, checkout destructively, rebase, or clean the worktree.
5. Inventory the complete tracked tree and identify all workspace members/crates, frontend packages, Tauri configuration/capabilities, test directories, migrations, CI workflows, installer definitions, docs, ADRs, and evidence reports.
6. Record versions: Windows/WSL distribution, kernel, architecture, Rust/Cargo, Node, package manager, Tauri CLI, WebView2 where discoverable, SQLite, and relevant toolchain.
7. Record hashes of lockfiles and build manifests (Cargo.lock, pnpm-lock.yaml/package-lock.json/yarn.lock, Cargo.toml, package.json, tauri.conf.json).
8. Identify existing scripts and test commands by reading manifests; do not guess commands.
9. Record existing user modifications and untracked files. Do not stage or modify anything.
10. Establish per-agent isolation requirements and external audit output directory.

## Commands

Use read-only equivalents where possible:

```bash
git remote -v
git branch --show-current
git rev-parse HEAD
git status --short
git log -20 --oneline --decorate
git ls-files
rustc --version
cargo --version
node --version
npm --version
pnpm --version
sqlite3 --version
```

A missing executable is recorded as unavailable, not installed.

## Required outputs

- `R2-EXEC-01-baseline-report.md`
- `R2-EXEC-01-repository-inventory.md`
- `R2-EXEC-01-environment-manifest.md`
- `R2-EXEC-01-evidence-index.md`
- machine-readable `audit-baseline.json`

## Acceptance criteria

- Repository identity and exact audited SHA are explicit.
- Local dirty state is captured before any test execution.
- All subsequent packages can use the same SHA.
- Toolchain and lockfile hashes are recorded.
- Inventory distinguishes source, tests, docs, prototypes, and generated artifacts.
- No tracked or user file is modified.

## Stop conditions

Stop if repository identity is uncertain, baseline cannot be agreed, user changes may be overwritten, or safe isolation cannot be established. Report BLOCKED with evidence; do not attempt automatic cleanup.
