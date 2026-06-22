# witness — Design Specification

## Purpose

Provide reproducible, tamper-evident records of command execution for AI agent workflows. Agents need to prove that tests passed, deploys ran, or builds succeeded — especially across session boundaries where context is lost.

## Architecture

```
cli.rs        — Argument parsing (clap derive API)
capture.rs    — Command execution + environment/git collection
store.rs      — Filesystem persistence + integrity verification
report.rs     — Human-readable and JSON output formatting
main.rs       — Dispatch + error handling
```

## Data Model

### Evidence (core struct)

```rust
struct Evidence {
    id: String,           // SHA-256(timestamp + pid)[..12]
    timestamp: String,    // RFC 3339
    command: String,      // Full command string
    tag: Option<String>,  // User-provided label
    cwd: String,          // Working directory at execution
    exit_code: i32,       // Process exit status
    duration_ms: u128,    // Wall-clock execution time
    stdout: String,       // Full captured stdout
    stderr: String,       // Full captured stderr
    environment: Environment,
    git_context: Option<GitContext>,
    bundle_hash: String,  // SHA-256 integrity hash
}
```

### Bundle Hash Computation

```
SHA-256(command || timestamp || exit_code || stdout || stderr)
```

Fields are concatenated as UTF-8 bytes. The hash covers the minimum fields needed to detect tampering while being reproducible from the stored data.

### ID Generation

```
SHA-256(timestamp || process_id)[..12]
```

12 hex chars = 48 bits of entropy. Sufficient for local evidence stores (collision probability negligible under millions of records).

## Storage Layout

```
<repo>/.agent-witness/
  .gitignore          # Contains "*" — never committed
  evidence/
    <id>.json         # Pretty-printed Evidence struct
```

## Verification Protocol

1. Load evidence bundle from disk
2. Recompute SHA-256 over (command, timestamp, exit_code, stdout, stderr)
3. Compare computed hash to stored `bundle_hash`
4. Return pass/fail

This catches:
- Manual edits to output fields
- Corrupted writes
- Intentional result falsification

This does NOT catch:
- Wholesale replacement of entire bundle (id + hash)
- Replay attacks (same command produces different results)

## Error Handling

All errors are typed via `WitnessError` enum with structured JSON output when `--format json` is active. Exit codes are deterministic per error variant.

## Design Decisions

1. **No database** — JSON files are inspectable, diffable, portable. One file per evidence bundle.
2. **Gitignored by default** — Evidence contains secrets (env vars, output). Never committed.
3. **SHA-256 for hashing** — Industry standard, no custom crypto.
4. **Truncated display** — stdout/stderr capped at 20/10 lines in text mode. Full data in JSON mode.
5. **Git context optional** — Works outside git repos (git_context will be None).
6. **Process output captured, not streamed** — Tradeoff: no real-time output during recording, but complete capture.
