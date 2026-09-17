# Security Policy

## Supported versions

Gateflow is pre-1.0 (local-dev MVP). Security fixes go into the latest
`main`; older tags/branches are not patched.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| `< 1.0` tags | :x:           |

## Reporting a vulnerability

**Do not open a public issue.** Report privately via a GitHub
**Security Advisory** (Security tab → Advisories → New draft advisory), or
any other private channel the maintainers publish.

Include:

- Affected component and version/commit
- Steps to reproduce (minimal upstream + request that triggers it)
- Impact assessment (what an attacker gains: header leak, SSRF reach,
  DoS amplification, …)

What to expect: acknowledgment within 5 business days, a fix or mitigation
plan on `main`, and credit in the release notes if you want it.

## Scope notes (threat model, MVP)

- The gateway forwards client headers/body to operator-configured upstreams
  by design. Treat `upstream_url` and static egress headers as privileged
  configuration — only trusted operators should create/edit workflows.
- `passthrough_headers` forwards client-supplied values upstream; keep it
  minimal (default: `authorization`, `accept`, `content-type`).
- No authentication on the control-plane in the MVP — do not expose
  `:8080` to untrusted networks. Bind to loopback (`GATEFLOW_BIND`) or put
  it behind an authenticated reverse proxy.
- SQLite files (`*.db*`) may contain request metadata (never bodies or keys
  by design) — they are git-ignored; keep backups access-controlled.
