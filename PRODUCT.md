# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users
Platform teams operating AI gateway infrastructure, and solo developers self-hosting their own LLM traffic — both managing streaming requests in dev and prod.

## Product Purpose
Self-deployed streaming workflow gateway. MVP is Ingress → Egress passthrough with negligible added latency; the engine stays generic so future node kinds unlock gateway-level possibilities: tool injection, skill loading, provider pooling, egress IP control, and many more. Success = byte-identical streamed output vs direct upstream with minimal overhead, plus a node system expressive enough to carry those futures.

## Positioning
Generic streaming engine where gateway capabilities compose as nodes — not a single-provider proxy or fixed middleware chain. MVP proves the hot path; architecture carries future node kinds via StreamNode + NodeFactory without executor changes.

## Operating Context
- Workflows: create → builder (ReactFlow canvas) → deploy (validate + compile to DAG cache) → test stream → `ANY /flow/:slug` live traffic
- Local dev ports: gateway 8080 (`GATEFLOW_BIND`), mock upstream 9099, web 3000 (`NEXT_PUBLIC_API_URL`)
- Tools: Rust gateway (Axum control-plane + data-plane), Next.js 16 builder talking directly to Rust, SQLite + in-memory compiled-DAG cache, mock SSE upstream
- MVP rules (per README/AGENTS.md): exactly 1 Ingress, 1 Egress, linear chain

## Capabilities and Constraints
- Confirmed: Ingress → Egress SSE passthrough; `/api/workflows` CRUD + deploy; `ANY /flow/:slug` data-plane
- Explicitly undecided: user selected "none" on durable constraints — latency budget (<1ms p50), zero-copy Bytes rule, linear-chain rule, and no-Next-handler rule are documented in README/AGENTS.md but NOT confirmed as binding for future work

## Brand Commitments
Name Gateflow. No confirmed voice, logo, or identity constraints.

## Evidence on Hand
- README.md curl tour (create → deploy → `curl -N /flow/chat`), mock upstream `crates/server/examples/mock_upstream.rs`
- Live UI: `web/app/page.tsx` (workflow list), `web/app/builder/[id]/page.tsx` (ReactFlow canvas + test drawer)
- No testimonials, customers, benchmarks beyond local byte-identical check, pricing, or deployment claims — must not be fabricated.

## Product Principles
1. Hot-path latency is a feature, not a metric
2. Engine stays generic; new behavior = new node kind, not executor forks
3. Builder explains the stream, never sits in it
4. Minimal scoped diffs; verify by execution (`cargo test`, `tsc`, live `curl -N` comparison)

## Accessibility & Inclusion
No product-specific requirement established. Web builder inherits standard keyboard / screen-reader expectations — undecided.
