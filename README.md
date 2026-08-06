# witness

`witness` is a reproducible command evidence recorder. It runs a command,
captures stdout, stderr, exit code, duration, exact argv, environment, git
context, and a versioned bundle hash, then stores the result locally.

It answers:

```text
Can we prove what command was run and what it produced?
```

## Quickstart

```sh
cargo build

# Record a test run.
cargo run -- run -- cargo test

# Record with a tag.
cargo run -- run --tag test -- cargo test

# Browse and inspect evidence.
cargo run -- list
cargo run -- show <id>
cargo run -- verify <id>
cargo run -- doctor
```

After installation, replace `cargo run --` with `witness`.

## Commands

### run

```sh
witness run -- cargo test
witness run --tag lint -- cargo clippy
witness run --format json --tag smoke -- sh -c "printf hello"
```

`witness run` records the wrapped command's exit code as evidence. If the
wrapped command fails, `witness` still succeeds as long as it captured and
stored the evidence.

### list

```sh
witness list
witness list --limit 5
witness list --format json
```

`list` reports valid evidence and surfaces unreadable or invalid local bundles
instead of silently hiding them. JSON output includes `invalid_count` and an
`invalid` array with bundle paths and parse/read reasons.

### show

```sh
witness show <id>
witness show <id> --format json
```

### verify

```sh
witness verify <id>
witness verify <id> --format json
```

JSON verification includes `verified` plus a stable `reason`: `valid`,
`hash_mismatch`, or `unsupported_hash_version`.

### doctor

```sh
witness doctor
witness doctor --strict
witness --format json doctor
```

Checks local evidence-store readiness without creating `.agent-witness`.
JSON output uses `schema_version: witness.doctor.v1` and includes
`status`, `action_level`, `gates`, `invalid_count`, `latest_evidence`, and
structured `recommended_commands`.

`doctor --strict` prints the same report, then exits 0 for `none` and 30 for
`review` or `stop`.

## Storage

```text
.agent-witness/
  .gitignore
  evidence/
    <id>.json
```

The storage directory is ignored by default.

## Integrity

New evidence bundles use `schema_version: 2` and
`bundle_hash_version: witness-v2`. The v2 hash protects the command display,
exact argv, tag, cwd, exit code, duration, stdout, stderr, environment, git
context, and hash contract version.

Older bundles without `bundle_hash_version` still verify with the legacy hash
contract, which covered command text, timestamp, exit code, stdout, and stderr.
Legacy verification is kept for compatibility, but new evidence should use v2.

Evidence with an unknown explicit hash contract fails closed. `show` remains a
read-only inspection command and will render malformed hash previews safely;
use `verify` to determine whether a bundle is trusted.

## Typical Agent Flow

```sh
probe doctor
sieve analyze

# Run targeted tests with evidence.
witness run --tag test -- cargo test --test cli_claims

# Run full validation with evidence before handoff.
witness run --tag full-test -- cargo test

# Include evidence IDs in the final note or latch handoff.
witness doctor
witness list
```

## Design

The implementation contract is in [docs/SPEC.md](docs/SPEC.md).
