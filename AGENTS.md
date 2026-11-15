# AGENTS.md - shopcraft

Local-first physical shop POS, cash drawer (Kassa), Z-Bon, and thermal receipt engine (`shopcraft`).

## Layout

- `crates/shopcraft-core` (L1): SQLite WAL store (POS catalog with EAN-13 barcodes, shifts, immutable receipts & `z_bons`, split-tender checkout, storno, 42-col ESC/POS receipt formatter), integer-cent `Money` math, `ToolHost` dispatch.
- `crates/shopcraft-mcp` (L2): `shopcraft` binary. CLI (`--json`) and MCP `2024-11-05` stdio server.
- Shared kernel: `../craft-biz` (`Money`, `Bus`, `mcp`).

## Invariants

- Currency uses integer cents (`i64`); never use floats for prices, tax, tender, or change.
- `receipt_items` and `z_bons` are immutable (`BEFORE UPDATE` and `BEFORE DELETE` SQLite triggers). Corrections use `storno`.
- Zero-panic: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` denied outside tests.
- No em dash characters in code, docs, or commit messages.

## Gates

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
2. `cargo test --workspace`
3. `cargo fmt --all --check`
