# PROJECT.md — witness

**What:** Reproducible command evidence recorder. Wraps commands, captures full execution context, stores auditable evidence bundles with integrity hashes.

**Status:** MVP complete. Run/list/show/verify, docs, and integration tests are complete (25 tests). Evidence bundles now use a versioned integrity contract for new captures, list surfaces corrupt local bundles, and verify reports stable machine-readable reasons. Shared plumbing (repo resolution, `--format`, exit codes, error report) comes from `agent-tools-core`; `run --propagate-exit` forwards the wrapped command's exit code for gates.

**Tech:** Rust 2021, clap 4, serde/serde_json, chrono, sha2, thiserror, agent-tools-core (path dep).

**Dependency note:** `agent-tools-core` is a path dependency (`../agent-tools-core`). A standalone clone needs that repo checked out beside this one until Mark decides to publish the crate (crates.io or git dep).

**Storage:** `.agent-witness/evidence/<id>.json` under repo root, gitignored.

## Module Ownership

| Module | Owner | Status |
|--------|-------|--------|
| cli.rs | Nix | Done |
| main.rs | Nix | Done |
| capture.rs | Nix | Done |
| store.rs | Nix | Done |
| report.rs | Nix | Done |
| docs/SPEC.md | Bjarn | Done |
| README.md | Bjarn | Done |

## Usage

```sh
witness run -- cargo test                  # record a test run
witness run --tag deploy -- ./deploy.sh    # tagged evidence
witness run --propagate-exit -- cargo test # exit with the wrapped command's code (git hooks)
witness list                               # browse recent evidence
witness show <id>                          # full evidence detail
witness verify <id>                        # check bundle integrity
witness doctor                             # evidence-store readiness
```

## Evidence Bundle Contents

- Command executed
- Exact command argv
- Exit code and duration
- Full stdout/stderr
- Environment: OS, user, rust/node versions
- Git context: branch, HEAD SHA, dirty state
- Timestamp (RFC3339)
- Bundle hash (SHA-256 over the declared hash contract)

## Last Updated

2026-09-11 — Moved repo resolution, `--format`, exit codes, and the stderr
error report onto `agent-tools-core`. Added `run --propagate-exit` (opt-in:
the documented default of exiting 0 after a failed wrapped command is pinned
by SPEC.md and existing tests, so the default was not flipped — Mark's call)
plus `--version` and propagate tests. `cargo test` passes with 25 tests.

2026-08-06 — Added `witness.doctor.v1` evidence-store preflight with
status/action_level/gates, invalid-bundle review action, latest evidence
summary, structured recommended commands, and strict gate exits.

2026-08-06 — Added `witness-v2` evidence hashing for full-context integrity, corrupt-bundle list reporting, safe hash previews, and verify reason codes.
