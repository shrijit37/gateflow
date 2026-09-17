# Contributing to Gateflow

Thanks for helping out. This guide gets you from clone to green CI.

## Prerequisites

- Rust stable toolchain via [rustup](https://rustup.rs) (`rustc`, `cargo`,
  `clippy`, `rustfmt`).
- Node 24 + npm (see `web/.nvmrc`; `nvm use` works).
- No database services needed — SQLite file + in-memory cache only.

## Setup

```bash
git clone <repo-url> && cd gateflow
cargo build --workspace
cd web && npm ci && cd ..
cp /dev/null /tmp/noop  # nothing to copy: config is env vars with sane defaults
```

Run the stack (three terminals):

```bash
cargo run -p gateflow-server --example mock_upstream   # :9099
cargo run -p gateflow-server                           # :8080 (GATEFLOW_BIND to override)
cd web && npm run dev                                  # :3000
```

Follow the 30-second curl tour in `README.md` to verify your setup.

## Workflow

1. Open an issue first for anything beyond a trivial fix (or explain in the
   PR why there is none).
2. Branch from `main`: `feat/<slug>`, `fix/<slug>`, `docs/<slug>`.
3. Keep the hot path zero-copy: no parsing/buffering in
   `crates/server/src/dataplane.rs` or `EgressNode::run`. Data-plane PRs must
   include a byte-identical + timing comparison (see README).
4. Run the checks in `AGENTS.md` (tests, clippy, fmt, tsc, eslint) before
   pushing. `next build` must pass for UI changes.
5. Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`…).
6. Open a PR against `main` using the template; keep CI green; one approval
   merges (squash-merge).

## Style

- Rust: `cargo fmt` defaults; workspace deps in root `Cargo.toml`.
- TS: eslint (Next config) + existing `@layer components` classes.
- Markdown: wrap prose ~80 cols where readable; fenced code blocks with
  language tags.

## Reporting bugs / requesting features

Use the issue templates (`.github/ISSUE_TEMPLATE/`). Include versions
(`rustc --version`, `node --version`), repro steps, and — for streaming bugs —
a minimal upstream + the differing bytes.

## Security

Do **not** open public issues for vulnerabilities — see `SECURITY.md`.

## Code of conduct

Be kind and professional: `CODE_OF_CONDUCT.md` applies everywhere in this
project (issues, PRs, discussions).
