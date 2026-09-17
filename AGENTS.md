# AGENTS.md — instructions for coding agents working in this repo

Read this before touching code. `README.md` covers product overview and the
curl tour; this file covers how to work here without breaking things.

## What this is

Streaming workflow gateway. MVP = `Ingress → Egress` passthrough with
negligible added latency. Engine must stay generic for future
convert / tool / skill / guardrail nodes.

## Layout

```
Cargo.toml                  # workspace deps (pin versions here, not in crates)
crates/engine/              # DAG validate/topo-sort, StreamNode trait, executor, StreamFrame IR
crates/protocol/            # detect() + SSE splitter (observability only)
crates/server/              # Axum control-plane + ANY /flow/:slug data-plane + sqlx + DAG cache
  migrations/               # additive SQL only, sqlx::migrate! picks them up
  examples/mock_upstream.rs # :9099 mock SSE upstream
web/                        # Next.js 16 App Router; talks DIRECTLY to Rust (no route handlers in hot path)
  app/page.tsx              # workflow list
  app/builder/[id]/page.tsx # ReactFlow canvas + panels + test drawer
  components/               # flow-nodes, IngressPanel, EgressPanel, TestDrawer
  lib/                      # api.ts (typed client), workflow-store.ts (zustand)
```

## Ports (local dev)

Gateway `8080` (`GATEFLOW_BIND` to override), mock upstream `9099`,
web `3000` (`NEXT_PUBLIC_API_URL` points web at the gateway).

## Commands (run before finishing any change)

```bash
cargo test --workspace                       # Rust unit tests (must be green)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cd web && ./node_modules/.bin/tsc --noEmit   # TS types (must be clean)
cd web && ./node_modules/.bin/eslint app lib components
```

Post-edit hooks (`.opencode/hook/hooks.yaml`, git-ignored local config) run
`cargo check` / `tsc` automatically — they are a backstop, not a substitute
for the full commands above. `next build` must pass before UI PRs merge.

## Rust conventions

- Edition 2024, workspace `Cargo.toml` owns all dependency versions.
- Hot path (`dataplane.rs`, `EgressNode::run`) is **zero-copy `Bytes`**:
  never parse/re-serialize bodies, never buffer full streams, flush per chunk.
- Errors: `thiserror` (`NodeError`/`DagError`) on fallible paths with `?`;
  `anyhow` at boundaries (db, main). `.unwrap()` only where infallible by
  construction (guarded map lookups, mutex locks) — never on I/O or parsing.
- Timeouts come from `ExecCtx::deadline`; aborts via `ExecCtx::abort`
  (`CancellationToken`). Never invent a second timeout mechanism.
- `tracing` for logs; never log request bodies or auth headers.
- Engine stays protocol-agnostic: new node kinds implement `StreamNode` and
  get wired through `NodeFactory` — no executor changes for new kinds.

## TypeScript conventions

- `'use client'` on interactive components; server components only for static shell.
- All gateway I/O goes through `lib/api.ts`. Never add Next.js route
  handlers to the streaming path.
- ReactFlow v12 (`@xyflow/react`): custom nodes in `components/flow-nodes.tsx`,
  edge rule (ingress → egress only) mirrored in `isValidConnection` AND the
  engine validator — keep both in sync.
- Shared form styles live in `app/globals.css` (`@layer components`).
- No `setState` directly in effects; remount panels via `key={node.id}`.

## SQLite

Schema changes = new additive migration file. `sqlx` uses runtime-checked
queries — no compile-time DB needed, CI needs no services.

## Commits & PRs

- Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, …).
- PRs must link an issue (or explain why none), keep CI green, and note
  latency impact for any data-plane change (byte-identical + timing comparison).
- Never commit: `.opencode/`, `opencode.json*`, agent session/transcript
  files, `.env*`, `*.db*`, `target/`, `node_modules/`, `.next/`
  (all git-ignored; CI rejects tracked agent artifacts).
