# AGENTS.md - shopcraft

Eight storefront modes in one repo (`shopcraft`): retail, web, amazon, subscription, digital, wholesale, ticket, rental.

## Layout

- `crates/shopcraft-core` (L1): `suite` dispatches mode-prefixed tools. Retail stays in `store.rs`. The other seven modes live under `src/modes/`.
- `crates/shopcraft-mcp` (L2): `shopcraft` binary. `--mode` selects the CLI storefront. `mcp` serves every mode on one stdio server.
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
