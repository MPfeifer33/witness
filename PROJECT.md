# PROJECT.md — witness

**What:** Reproducible command evidence recorder. Wraps commands, captures full execution context, stores auditable evidence bundles with integrity hashes.

**Status:** MVP complete. Run/list/show/verify, docs, and integration tests are complete.

**Tech:** Rust 2021, clap 4, serde/serde_json, chrono, sha2, thiserror.

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
witness list                               # browse recent evidence
witness show <id>                          # full evidence detail
witness verify <id>                        # check bundle integrity
```

## Evidence Bundle Contents

- Command executed
- Exit code and duration
- Full stdout/stderr
- Environment: OS, user, rust/node versions
- Git context: branch, HEAD SHA, dirty state
- Timestamp (RFC3339)
- Bundle hash (SHA-256 over key fields)

## Last Updated

2026-06-22 — MVP complete; `cargo test` passes with 12 integration tests.
