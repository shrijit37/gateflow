# Future Nodes

The engine is protocol-agnostic and generic: new node kinds implement
`StreamNode` and get wired through `NodeFactory` — no executor changes for
new kinds. This file tracks what's planned.

## Current state

- **Implemented**: `ingress` (pass-through), `egress` (streams request
  body up, re-streams response body down with a `Head` frame).
- MVP rules enforced by engine + UI: exactly 1 Ingress, 1 Egress, linear
  chain, Ingress → Egress only.
- `NodeKind` already reserves the following variants (see
  `crates/engine/src/node.rs`).

## Planned node kinds (phase 2)

| Kind | Purpose | Notes |
|---|---|---|
| `convert` | Protocol/format transformation between nodes | `StreamFrame::Event`/`Head` exist now so converters slot in without engine changes |
| `tool` | Call external tools / APIs mid-stream | Shares the `reqwest::Client` pattern from egress |
| `skill` | Inject reusable sub-workflow logic | Compose smaller DAGs |
| `guardrail` | Filter/modify frames in-flight (PII, moderation, policy) | Pure stream transformation, zero-copy preferred |
| `cache` | Cache upstream responses or partial streams | Cache-hit responders can emit `Head` themselves |

## Wiring a new node (checklist)

1. Add/confirm the `NodeKind` variant in `crates/engine/src/node.rs`.
2. Implement `StreamNode` in `crates/server/src/node_registry.rs`.
3. Register it in `Registry::build` (the `NodeFactory` impl).
4. Mirror `isValidConnection` / engine validator edges in the UI
   (`components/flow-nodes.tsx`, `web/`).
5. Add an additive migration if the node has persisted config.

## Notes

- Hot path stays zero-copy `Bytes`: never parse/re-serialize bodies in the
  data plane.
- Timeouts come from `ExecCtx::deadline`; aborts via `ExecCtx::abort`
  (`CancellationToken`).
- Per AGENTS.md: Data-plane changes must note latency impact (byte-identical
  + timing comparison).