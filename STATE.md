# STATE.md

Truthful current state. This file wins over any other doc in this repo when
they disagree. Last verified **2026-10-10** (main at `c574c4b`).

## Live endpoints

**None. Gateflow is not deployed.** There is no production URL, no domain, no
deploy pipeline and no Dockerfile in this repo. `.github/workflows/ci.yml` is
the only workflow, and it has two jobs: `rust` and `web`.

If it ever needs a production deployment, the pieces that are missing are a
Dockerfile for the `server` crate (it needs the SQLite file on a writable
volume, so an ephemeral-filesystem container would lose state), a
`deploy-prod` job matching the fleet shape, and a decision about where the
SQLite file lives — a single-node deploy is fine, a second replica is not.

## Class

`UNDEPLOYED` — buildable and CI-green, not serving anything.

## What it is

Rust workspace: `crates/engine`, `crates/protocol`, `crates/server` (axum 0.8,
tokio 1.53, sqlx 0.9) plus a Next.js 16.4 / React 19.3 app in `web/`. It is an
LLM gateway with a policy/workflow engine.

| Config | Env var | Default |
|---|---|---|
| Listen address | `GATEFLOW_BIND` | `0.0.0.0:8080` |
| Database | `DATABASE_URL` | `sqlite://gateflow.db` (WAL, `synchronous=NORMAL`) |

## Health contract (in place, unexercised in production)

`GET /health` on the server returns exactly `{status, service, version,
checks}` with `service: "gateflow-api"` and a real `SELECT 1` in
`checks.database`, answering 503 when the database is unreachable
(`crates/server/src/routes.rs`). `web/app/health/route.ts` serves the static
contract with `service: "gateflow-web"` and `checks: { assets: "up" }` — a
static route has no runtime dependency, so claiming the assets are up is the
honest answer. Both were added in `c574c4b`, together with the Next 16.4 and
React 19.3 bumps.

## Do not merge dependabot PR #4 (typescript 5.9.3 → 7.0.2 in /web)

It breaks `web` linting, and not because of anything gateflow-specific:
`typescript-eslint@8.71.1` declares
`peerDependencies: { eslint: "^8.57.0 || ^9.0.0 || ^10.0.0", typescript: ">=4.8.4 <6.1.0" }`.
TypeScript 7 is outside that range, so the whole fleet stays on TypeScript
5.9.x until typescript-eslint widens its range. This is a deliberate deferral,
not an oversight. PR #6 (`@types/node` 24 → 26) is the same class of
premature bump; leave it too.

## CI

Green on `main` at `c574c4b`. The newest red run in the Actions tab
(`ab5e2b1`) belongs to the #4 dependabot branch, not to `main`.