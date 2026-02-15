# AGENTS.md — SQ

## What This Is
Phext sync server (Rust). REST API for reading/writing scrolls. Multi-tenant auth.

## Rules
- Pull before touching code: `git pull --rebase origin exo`
- Read modified files after pull before editing them
- Don't stomp on siblings' active work — coordinate first
- Commit messages: describe what changed, not why you exist

## Validation
`cargo test && cargo build --release`

## Branch
Default: `exo`
