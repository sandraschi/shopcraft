# ShopCraft

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

## Install

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

One repo, eight storefront modes, one SQLite file per mode under a data directory.

## Feature matrix

| Feature | Status | Where |
|---|---|---|
| Retail shift, split tender, storno, Z-Bon, thermal preview | Implemented | `--mode retail` (default), tools `shop_*` and `retail_*` |
| Web catalog, promo, shipping zone, order pay/ship | Implemented | `--mode web`, tools `webshop_*` |
| Amazon unit economics and local Buy Box clamp | Implemented | `--mode amazon`, `amazon_reprice` clamps to min/max from a supplied competitor price. No Amazon API. |
| Subscription cadence, skip, pause, swap, IBAN stored | Implemented | `--mode subscription`, tools `sub_*` |
| SEPA pain.008 file | Planned | Mandate reference is stored. No pain.008 XML is written. |
| Digital license keys and per-country VAT rate at checkout | Implemented | `--mode digital`. Keys are unique per order. |
| EU VAT OSS return filing | Planned | The rate table is local. Nothing is filed. |
| Wholesale MOQ, pack size, tier discount, reverse-charge quote | Implemented | `--mode wholesale`, `wholesale_quote` |
| Ticket capacity, check-in, refund | Implemented | `--mode ticket` |
| Rental overlap, checkout, late fee | Implemented | `--mode rental` |
| Live payment, carrier, or marketplace connectors | Planned | Every mode is offline SQLite. |

## Modes

`retail` `web` `amazon` `subscription` `digital` `wholesale` `ticket` `rental`

```powershell
cargo run -p shopcraft-mcp -- --mode retail seed
cargo run -p shopcraft-mcp -- --mode wholesale status
cargo run -p shopcraft-mcp -- --mode digital tool digital_catalog
cargo run -p shopcraft-mcp -- mcp
cargo run -p shopcraft-app
```

`--db` is a directory. Default `shopcraft-data`. The desktop app uses `%LOCALAPPDATA%\shopcraft`. The old single-file `shopcraft.db` is not migrated.

MCP exposes every mode on one stdio server. Retail tools keep the `shop_*` names and also answer as `retail_*`.

## Layout

| Layer | Crate | Role |
|---|---|---|
| L1 | `crates/shopcraft-core` | Shared suite plus one module per mode |
| L2 | `crates/shopcraft-mcp` | `shopcraft` CLI (`--mode`) and MCP stdio |
| L3 | `crates/shopcraft-app` | egui window with a mode switcher |

## Verification

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## License

MIT OR Apache-2.0
