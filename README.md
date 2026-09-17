# Gateflow — streaming workflow gateway

n8n-like workflow app specialized **only** in end-to-end streaming request
traffic. MVP is `Ingress → Egress` passthrough with negligible added latency;
the engine is built for future converter / tool / skill / guardrail nodes.

```
browser / curl ──► ANY /flow/:slug ──► gateway ──► upstream SSE ──► stream back
                         │                            (zero-copy Bytes,
                    control-plane:               no parse/re-serialize)
                    /api/workflows CRUD + deploy
```

## Stack (latest stable, pinned 2026-09-18)

| Layer | Tech |
|---|---|
| Engine + gateway | Rust workspace: `axum 0.8`, `tokio 1.53`, `reqwest 0.13`, `sqlx 0.9` (SQLite + in-memory compiled-DAG cache) |
| Frontend | Next.js 16, React 19, `@xyflow/react` 12, Zustand 5, Tailwind 4 — talks **directly** to Rust, no Next route handlers in the hot path |
| Dev tooling | `cargo check` + `tsc --noEmit` post-edit hooks (`.opencode/hook/hooks.yaml`), LSP (`rust-analyzer`, `typescript-language-server`) in `opencode.json` |

## Quickstart (local dev)

```bash
# 1. backend (uses 8081 here because 8080 is taken on this machine;
#    default is 8080 — override with GATEFLOW_BIND)
cargo run -p gateflow-server --example mock_upstream   # :9099 mock SSE upstream
GATEFLOW_BIND=127.0.0.1:8081 cargo run -p gateflow-server

# 2. frontend
cd web && npm install && npm run dev                   # :3000
```

Open http://localhost:3000 → **New workflow** → **Open builder** →
**Deploy** → **Test stream** → **Send**.

## The 30-second curl tour

```bash
BASE=http://127.0.0.1:8081

# create
curl -s -X POST $BASE/api/workflows -H 'content-type: application/json' -d '{
  "name": "chat",
  "slug": "chat",
  "definition": {
    "nodes": [
      {"id":"in","kind":"ingress","name":"Ingress",
       "config":{"method":"POST","protocol":"auto","timeout_ms":120000}},
      {"id":"out","kind":"egress","name":"Egress",
       "config":{"upstream_url":"http://127.0.0.1:9099/chat","method":"POST",
                 "headers":{},"passthrough_headers":["authorization"],
                 "forward_response_headers":["content-type"],
                 "connect_timeout_ms":null,"timeout_ms":null}}
    ],
    "edges": [{"from":"in","to":"out"}]
  }
}'

# deploy (validates + compiles into the hot-path cache)
ID=$(curl -s $BASE/api/workflows | python3 -c 'import sys,json; print(json.load(sys.stdin)[0]["id"])')
curl -s -X POST $BASE/api/workflows/$ID/deploy

# stream through the gateway (OpenAI-shaped body, auto-detected)
curl -N -X POST $BASE/flow/chat -H 'content-type: application/json' -d '{
  "model": "gpt-4o-mini", "stream": true,
  "messages": [{"role": "user", "content": "Say hello as a stream"}]
}'
```

Verified: gateway output is **byte-identical** to the upstream stream
(~8 ms total overhead on a 770 ms / 20-chunk stream, incl. SQLite logging).

## Repo layout

```
Cargo.toml                  # workspace: engine, protocol, server
crates/engine/              # DAG validate/topo-sort, StreamNode trait, executor, StreamFrame IR
crates/protocol/            # detect() (openai/anthropic/gemini/raw) + SSE splitter (observability only)
crates/server/              # Axum control-plane + ANY /flow/:slug data-plane + sqlx + DAG cache
  migrations/001_init.sql   # workflows + executions tables
  examples/mock_upstream.rs # :9099 mock SSE upstream (/chat, /slow, /ping)
web/                        # Next.js: list page, ReactFlow builder, live test drawer
.opencode/hook/hooks.yaml   # post-edit compiler gates
opencode.json               # LSP servers
```

## Design notes

- **Hot path never parses**: detection uses headers + ≤4 KiB body-prefix peek;
  frames flow as `Bytes`, flushed per chunk. `StreamFrame::Event/Head`
  variants exist now so phase-2 converters slot in without engine changes.
- **MVP rules** (enforced by engine + UI): exactly 1 Ingress, 1 Egress,
  linear chain, Ingress → Egress only. Translation/converter nodes are phase 2.
- **Latency budget**: p50 overhead <1 ms, p99 <5 ms vs direct upstream
  (excluding upstream time) — verify with the comparison above, not by assertion.

## Env knobs (`crates/server/src/config.rs`)

`GATEFLOW_BIND` (default `0.0.0.0:8080`), `DATABASE_URL`
(default `sqlite://gateflow.db`), `GATEFLOW_CORS_ORIGINS`,
`GATEFLOW_BODY_LIMIT_MB` (100), `GATEFLOW_PEEK_BYTES` (4096),
`GATEFLOW_CONNECT_TIMEOUT_MS` (10000), `GATEFLOW_TIMEOUT_MS` (120000).

## Contributing & governance

- Workflow: `CONTRIBUTING.md` (agent working agreement: `AGENTS.md`, `CLAUDE.md`)
- Conduct: `CODE_OF_CONDUCT.md` · Security: `SECURITY.md`
- License: MIT — see `LICENSE`.
