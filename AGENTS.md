# AGENTS.md - shopcraft

Eight storefront modes in one repo (`shopcraft`): retail, web, amazon, subscription, digital, wholesale, ticket, rental.

## Layout

- `crates/shopcraft-core` (L1): `suite` dispatches mode-prefixed tools. Retail stays in `store.rs`. The other seven modes live under `src/modes/`.
- `crates/shopcraft-mcp` (L2): `shopcraft` binary. `--mode` selects the CLI storefront. `mcp` serves every mode on one stdio server.
- `crates/shopcraft-app` (L3): egui desks. **Shop > Mode** picks the shop. The sidebar lists that shop's sections. **Shop > Open webshop** publishes `storefront\` plus `catalog.js` under `%LOCALAPPDATA%\shopcraft\web-storefront`. The same button is on Web stats.
- `storefront\`: HTML shop. Product photos are `storefront\assets\*.jpg`. Browser checkout does not write SQLite.
- Shared kernel: `../craft-biz` (`Money`, `Bus`, `mcp`, `craft-biz-ui`).

## Retail floor

Retail sidebar groups are Floor (Stats, Register, Catalog), Shop (Hours, Calendar, Roster), and Till (Shift).

`retail.db` tables, written by `PosStore` and read by the window:

- `opening_hours`: weekday 0 = Monday through 6 = Sunday, `open_minute` and `close_minute` from midnight, `closed`.
- `calendar_days`: `day` is `YYYY-MM-DD`. `kind` is `closed`, `short`, or `open`.
- `employees`: `EMP-###`, name, role, active. Deactivate updates the row. It does not delete it.

`seed_demo` also calls `seed_floor`: standard week (Mon-Fri 10:00-18:00, Sat 10:00-16:00, Sun closed), calendar rows 2026-12-24 closed, 2026-12-26 closed, 2026-12-31 short 10:00-14:00, and EMP-001 Lukas Berger, EMP-002 Nora Berg, EMP-003 Mira Hofer.

These three are store methods. They are not `shop_*` or `retail_*` tools. Do not document a CLI flag for them.

## Sales chart

Every shop sidebar ends with **Sales**. `sales_by_day` on that mode's store groups its own rows by `YYYY-MM-DD`. The window draws bars, or an area line once a series is 24 points or longer. Quiet days are inserted only when the first and last day are at most 92 days apart. The series is not an MCP tool.

## Stock and chat

- Each mode has its own SQLite quantity. A sale does not change stockcraft on-hand.
- Bus files land in `%LOCALAPPDATA%\craft-biz\bus\stockcraft\inbox`. Stockcraft's drain rejects `channel.sale_committed`, `channel.return_restocked`, and `channel.stock_reserved`.
- Reorder and goods receipt belong to stockcraft (`stock_reorder_plan`, `stock_receive`).
- The window does not implement `chat_panel`. Do not document a chat dock.

## Invariants

- Currency uses integer cents (`i64`); never use floats for prices, tax, tender, or change.
- `receipt_items` and `z_bons` are immutable (`BEFORE UPDATE` and `BEFORE DELETE` SQLite triggers). Corrections use `storno`.
- Zero-panic: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` denied outside tests.
- No em dash characters in code, docs, or commit messages.

## Gates

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
2. `cargo test --workspace`
3. `cargo fmt --all --check`
