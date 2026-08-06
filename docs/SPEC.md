# witness spec

Status: MVP implementation contract

`witness` is a reproducible command evidence recorder. It runs a command,
captures execution context and output, stores an evidence bundle, and verifies
the bundle hash later.

## Goals

- Make "tests passed" and similar claims auditable after compaction.
- Store command output with exact argv, environment, and git context.
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

Valid bundles are listed normally. Unreadable or unparseable `.json` files in
the evidence directory are reported as invalid bundle entries instead of being
silently ignored. The list command should not fail the whole operation because
one local bundle is corrupt; corruption is surfaced as data for the caller.

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

Recomputes the SHA-256 bundle hash from the bundle's declared hash contract.
New bundles use `witness-v2`; legacy bundles without a hash version use the
original MVP hash contract.

JSON verification returns a stable reason string:

- `valid`
- `hash_mismatch`
- `unsupported_hash_version`

## Evidence Schema

```json
{
  "schema_version": 2,
  "id": "12-char-hash",
  "timestamp": "2026-06-22T04:40:00Z",
  "command": "cargo test",
  "command_argv": ["cargo", "test"],
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
  "bundle_hash_version": "witness-v2",
  "bundle_hash": "sha256 hex"
}
```

`git_context` is `null` outside a git repository.

## Integrity Contract

New evidence is written with:

- `schema_version: 2`
- `bundle_hash_version: witness-v2`
- `command_argv` preserving the exact argv vector passed after `--`

The `witness-v2` hash protects:

- schema version
- evidence id
- timestamp
- command display string
- exact argv
- tag
- cwd
- wrapped command exit code
- duration
- stdout
- stderr
- captured environment
- captured git context
- bundle hash version

Older evidence without `bundle_hash_version` is treated as `legacy-v1` and
verified with the original MVP hash contract:

- command display string
- timestamp
- wrapped command exit code
- stdout
- stderr

Legacy compatibility keeps old local evidence readable, but agents should treat
new `witness-v2` evidence as the stronger trust surface.

Evidence with an unknown explicit `bundle_hash_version` fails verification
closed. Witness should only verify hash contracts it knows how to recompute.

`show` is an inspection surface, not a trust decision. It must render partial
or malformed evidence without panicking where deserialization still succeeds;
verification remains the source of truth for integrity.

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

## List JSON Output

```json
{
  "ok": true,
  "evidence": [],
  "invalid_count": 1,
  "invalid": [
    {
      "path": ".agent-witness/evidence/bad.json",
      "reason": "json error: expected value at line 1 column 1"
    }
  ]
}
```

`evidence` is preserved for compatibility. `invalid_count` and `invalid`
describe local evidence files that could not be read or parsed.

## Verify JSON Output

```json
{
  "ok": true,
  "evidence_id": "abc123def456",
  "verified": false,
  "reason": "hash_mismatch"
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
