# What & why

Closes #<!-- issue number, or why none -->.

## Changes

-

## Data-plane latency impact (required if touching the hot path)

- [ ] N/A (no data-plane change)
- [ ] Byte-identical output vs direct upstream: <!-- paste cmp result -->
- [ ] Timing: direct <!-- -->s vs gateway <!-- -->s over <!-- --> chunks

## Checklist

- [ ] `cargo test --workspace` green (or N/A)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (or N/A)
- [ ] `tsc --noEmit` + `eslint` clean (or N/A)
- [ ] `next build` passes for UI changes
- [ ] Docs updated (`README.md` / `AGENTS.md` if behavior changed)
- [ ] No secrets, `.env*`, `*.db*`, or agent session/state files included
