# witness

Reproducible command evidence recorder. Wraps any command, captures full execution context, and stores auditable evidence bundles with integrity hashes.

Built for AI agent workflows where you need to prove work happened: test runs, deployments, builds, migrations.

## Install

```sh
cargo install --path .
```

## Usage

```sh
# Record a test run
witness run -- cargo test

# Record with a tag
witness run --tag deploy -- ./deploy.sh

# List recent evidence
witness list
witness list --limit 5

# Show full evidence detail
witness show <id>

# Verify bundle integrity (detect tampering)
witness verify <id>

# JSON output for programmatic use
witness --format json run -- cargo build
witness --format json list
```

## Evidence Bundle

Each bundle captures:

| Field | Description |
|-------|-------------|
| id | SHA-256 derived unique identifier |
| timestamp | RFC 3339 execution time |
| command | Full command string |
| tag | Optional categorization label |
| cwd | Working directory |
| exit_code | Process exit status |
| duration_ms | Execution time in milliseconds |
| stdout/stderr | Full captured output |
| environment | OS, user, rust/node versions |
| git_context | Branch, HEAD SHA, dirty state |
| bundle_hash | SHA-256 over key fields for integrity |

## Storage

Evidence is stored in `.agent-witness/evidence/<id>.json` under the repo root, automatically gitignored.

## Verification

`witness verify <id>` recomputes the SHA-256 hash over the bundle's key fields and compares it to the stored hash. If they differ, the bundle may have been tampered with.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Validation or JSON error |
| 2 | IO error |
| 3 | Evidence not found |

## License

MIT
