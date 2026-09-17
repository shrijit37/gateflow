# CLAUDE.md — Claude Code working agreement for Gateflow

`AGENTS.md` is the source of truth for commands, conventions, and layout.
This file adds Claude-specific session norms.

- Plan first for anything touching the data-plane hot path
  (`crates/server/src/dataplane.rs`, `node_registry.rs`): state the latency
  impact before and after, with measured numbers, not assertions.
- Verify by execution: `cargo test --workspace` for Rust, `tsc --noEmit` +
  `eslint` for web, plus a live `curl -N` stream comparison for data-plane
  changes (byte-identical output vs direct upstream, per README tour).
- Keep diffs minimal and scoped; never reformat unrelated code
  (`cargo fmt` / eslint should be no-ops on files you didn't touch).
- Never commit secrets, `.env*`, `*.db*`, or agent session/state files —
  they are git-ignored and CI rejects them.
- If a request conflicts with `AGENTS.md`, say so and follow `AGENTS.md`.
