# witness

`witness` is a reproducible command evidence recorder. It runs a command,
captures stdout, stderr, exit code, duration, environment, git context, and a
bundle hash, then stores the result locally.

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

## Storage

```text
.agent-witness/
  .gitignore
  evidence/
    <id>.json
```

The storage directory is ignored by default.

## Typical Agent Flow

```sh
probe doctor
sieve analyze

# Run targeted tests with evidence.
witness run --tag test -- cargo test --test cli_claims

# Run full validation with evidence before handoff.
witness run --tag full-test -- cargo test

# Include evidence IDs in the final note or latch handoff.
witness list
```

## Design

The implementation contract is in [docs/SPEC.md](docs/SPEC.md).
