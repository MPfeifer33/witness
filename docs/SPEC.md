# witness spec

Status: MVP implementation contract

`witness` is a reproducible command evidence recorder. It runs a command,
captures execution context and output, stores an evidence bundle, and verifies
the bundle hash later.

## Goals

- Make "tests passed" and similar claims auditable after compaction.
- Store command output with environment and git context.
- Keep evidence repo-local and gitignored.
- Support simple listing, showing, and hash verification.

## Non-Goals

- Secure sandboxing.
- Tamper-proof append-only storage.
- Streaming long-running command output.
- Remote evidence upload.

## Storage

```text
.agent-witness/
  .gitignore
  evidence/
    <id>.json
```

`.agent-witness/.gitignore` contains `*` by default. Evidence is local session
state, not a product artifact.

## Commands

### run

```sh
witness run -- cargo test
witness run --tag test -- cargo test
witness run --format json --tag lint -- cargo clippy
```

Runs the command after `--`, records evidence, and exits successfully as long
as the command could be executed and the evidence could be stored. A failing
wrapped command is recorded with `passed: false`.

### list

```sh
witness list
witness list --limit 5
witness list --format json
```

Shows recent evidence bundles, newest first.

### show

```sh
witness show <id>
witness show <id> --format json
```

Shows one full evidence bundle.

### verify

```sh
witness verify <id>
witness verify <id> --format json
```

Recomputes the SHA-256 bundle hash from the command, timestamp, exit code,
stdout, and stderr.

## Evidence Schema

```json
{
  "id": "12-char-hash",
  "timestamp": "2026-06-22T04:40:00Z",
  "command": "cargo test",
  "tag": "test",
  "cwd": "/path/to/repo",
  "exit_code": 0,
  "duration_ms": 87,
  "stdout": "...",
  "stderr": "...",
  "environment": {
    "os": "linux",
    "user": "mpfeifer",
    "rust_version": "rustc 1.96.0",
    "node_version": "v24.0.0"
  },
  "git_context": {
    "branch": "master",
    "head_sha": "abc1234",
    "dirty": false
  },
  "bundle_hash": "sha256 hex"
}
```

`git_context` is `null` outside a git repository.

## Run JSON Output

```json
{
  "ok": true,
  "evidence_id": "abc123def456",
  "exit_code": 0,
  "duration_ms": 87,
  "passed": true
}
```

## Exit Codes

| Code | Meaning |
| ---- | ------- |
| `0` | Witness command completed and evidence operation succeeded |
| `1` | Validation or JSON error |
| `2` | IO error |
| `3` | Evidence not found |

The wrapped command exit code is data in the evidence bundle; it does not
become the `witness run` process exit code in the MVP.
