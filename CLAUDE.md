# CLAUDE.md

This file is read by Claude Code at the start of every session in this repo.

---

## About This Repo

Rust observability agent for homelab nodes. It dials out to the Go
control plane (separate repo) and holds one persistent bidirectional
gRPC stream; the wire contract is `proto/homelab/agent/v1/agent.proto`,
generated on this side by tonic-build with a vendored protoc. Phase 1 is
observability only (heartbeat, inventory, filesystem usage, SMART via
`smartctl --json`); phase 2 actuation is sketched but not started. The
design doc at `docs/design/agent-design.md` is the reference.

Invariants to preserve when editing:

- The agent never listens on a port and never mutates the node in
  phase 1. It runs unprivileged; smartctl is the only escalation.
- Proto evolution: never reuse field numbers, additive changes only,
  gate behavior on `agent_version` from Hello.
- Deploy target is `x86_64-unknown-linux-musl` (static, rustls). Don't
  add deps that break the static build (e.g. anything linking OpenSSL).
- CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and
  `cargo test`; keep all three passing.

---

## Developer Preferences

### Editor
- Primary: Vim
- AI editor: Cursor

### Shell
- zsh, minimal prompt

### Git & GitHub Workflow
- **Branch model:** `main` = latest release. `develop` = integration branch.
- Always branch from `develop`, never commit directly
- PRs always target `develop`
- `main` is only updated via CLI merge (`git merge --no-ff origin/develop`) by `/publish-release` — **never via a GitHub PR**. GitHub's merge button squash-merges by default, dropping ancestry and causing conflicts on the next release.
- Conventional commits: `feat:`, `fix:`, `docs:`, `chore:`

### Scripting Standards
- Shell scripts must pass `shellcheck`
- Use `set -euo pipefail`
- Scripts should be idempotent

### Rust Standards
- `cargo fmt` before committing; CI enforces `--check`
- Clippy clean with `-D warnings`
- Commit `Cargo.lock` (this is a binary crate)
- Prefer `anyhow` at binary edges, typed errors only where callers match on them

---

## Brand

This repo descends from [`amcheste/repo-template`](https://github.com/amcheste/repo-template), which is brand-aligned with [`@amcheste/brand`](https://github.com/amcheste/alanchester-brand). Badge colors (Hunter Green `#1F4D3A`, Ink `#0B0B0C`) match the brand by default.

When generating prose, follow the brand voice rules at [`voice.md`](https://github.com/amcheste/alanchester-brand/blob/main/docs/voice.md): no em dashes in prose, calibrated hedges over weak ones, lowercase eyebrows, numerical specificity. Hunter green is reserved for data, pivots, and the δ; don't use it as decoration.

For deeper brand integration (palette adoption, mark embedding, full theming sweep), paste [`docs/theming-prompt.md`](https://github.com/amcheste/alanchester-brand/blob/main/docs/theming-prompt.md) from the brand repo into a Claude Code session in this repo.

---

## Learned Preferences

<!-- Claude Code will suggest additions here as patterns emerge across sessions -->
