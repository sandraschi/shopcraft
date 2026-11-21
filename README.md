# ShopCraft

![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)
![Rust 1.90+](https://img.shields.io/badge/rust-1.90%2B-orange?logo=rust)
![MCP stdio](https://img.shields.io/badge/MCP-stdio-purple)
![ArtCraft harness](https://img.shields.io/badge/ArtCraft-harness-green)

One repo, eight storefront modes, one SQLite file per mode under a data directory.

## Install

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

Installs the Windows app and the CLI (Start Menu shortcut, CLI on the user PATH). `-RegisterClients` also registers the MCP exe with the IDEs that are installed. `-Clients cursor,claude-desktop` limits that list. `-Mcpb` installs the Claude Desktop extension bundle.

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
| Desktop desks for all eight modes | Implemented | `shopcraft-app`. **Shop > Mode** switches the shop. The sidebar is that shop's sections. |
| Sales over time | Implemented | **Sales** on every shop. Daily totals from that shop's own SQLite. |
| Retail weekly hours, calendar exceptions, employee roster | Implemented | Retail sidebar only. Tables `opening_hours`, `calendar_days`, `employees` in `retail.db`. The window calls the store. There are no `shop_*` tools for these three. |
| HTML web storefront with product photos | Implemented | Web stats **Open webshop**, or menu **Shop > Open webshop**. Photos live in `storefront\assets`. |
| Browser checkout writing the desk | Planned | The page shows the catalog. Checkout does not insert a SQLite order. |
| Stockcraft quantity update on a sale | Planned | The shop writes a bus file. Stockcraft's inbox drain rejects it. Reorder stays in stockcraft. |
| Chat in the desktop window | Planned | `chat_panel` is empty. There is no model picker. |
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

`--db` is a directory. Default `shopcraft-data`. The desktop app uses `%LOCALAPPDATA%\shopcraft` (`retail.db`, `web.db`, and one file per other mode). The old single-file `shopcraft.db` is not migrated.

MCP exposes every mode on one stdio server. Retail tools keep the `shop_*` names and also answer as `retail_*`.

`dist\shopcraft-app.exe` is the 16 November binary. The window described here is `target\debug\shopcraft-app.exe` from `cargo run -p shopcraft-app`.

## Desktop window

**Shop > Mode** chooses the shop. The left sidebar is that shop's own sections.

| Shop | Sidebar |
|---|---|
| Retail | Stats, Register, Catalog, Hours, Calendar, Roster, Shift, Sales |
| Web | Stats, Catalog, Orders, Shipping, Sales |
| Amazon | Stats, Listings, Orders, Replenish, Sales |
| Subscription | Stats, Plans, Subscribers, Sales |
| Digital | Stats, Catalog, Orders, Licenses, Sales |
| Wholesale | Stats, Accounts, Catalog, Orders, Sales |
| Ticket | Stats, Events, Tickets, Sales |
| Rental | Stats, Gear, Bookings, Sales |

Retail Hours is the weekly open and close. Weekday 0 is Monday and 6 is Sunday. Minutes are counted from midnight. **Install standard week** writes Monday to Friday 10:00-18:00, Saturday 10:00-16:00, and Sunday closed. Each day can move by 30 minutes, or be marked closed.

Calendar holds days that break that week. Kind is `closed`, `short`, or `open`. The window can add a closed day or a short day (10:00-14:00) for a `YYYY-MM-DD` date, and it can remove a row.

Roster is the people who work the shop. Add a name and a role. Deactivate keeps the row. Ids are `EMP-001` upward.

Shift opens and closes the till and writes a Z-Bon.

**Seed demo items** writes the catalog and, in the same call, the standard week, closures on 2026-12-24 and 2026-12-26, a short day on 2026-12-31, and three employees: Lukas Berger (Clerk), Nora Berg (Floor), Mira Hofer (Manager). A shop seeded before these tables existed needs Seed again, or the Hours, Calendar, and Roster buttons, before those sections have rows.

**Sales** is the last sidebar item on every shop. Each bar is one day. A span of up to 92 days also draws the quiet days as empty bars, so the axis is the calendar. Longer histories keep only the days that have sales and switch to an area line. Hover a day for the date, the amount, and the order count.

What the bar sums:

| Shop | Amount |
|---|---|
| Retail | Receipt totals, storno included as a negative |
| Web | Orders that are not cancelled or refunded |
| Amazon | Gross of orders that are not cancelled |
| Subscription | Billed cycles. A skip is left out |
| Digital | Checkout totals, VAT included |
| Wholesale | Trade orders that are not cancelled |
| Ticket | Tickets that are not refunded |
| Rental | Rental, late, and damage fees. The deposit is left out |

Empty catalogs need **Seed demo** on Stats. Counts belong to that shop's SQLite file. Shops do not share a catalog, a stock figure, or a customer.

## Web storefront

`storefront\` holds `index.html`, `styles.css`, `app.js`, and six product photos under `storefront\assets\`. **Open webshop** copies that page next to a live `catalog.js` in `%LOCALAPPDATA%\shopcraft\web-storefront` and opens it in the browser. Product cards and the hero use those photos. Search, bag, and theme controls stay small icons.

If the web database has no products, the page shows its built-in sample and the status line says so.

## Stock

A web or retail sale changes only that mode's own quantity. The web **Order 1** button turns off at 0, and checkout refuses an oversell.

The shop also drops a JSON event in `%LOCALAPPDATA%\craft-biz\bus\stockcraft\inbox`:

| Shop action | Event kind | Source name |
|---|---|---|
| Retail checkout | `channel.sale_committed` | `shopcraft` |
| Retail storno | `channel.return_restocked` | `shopcraft` |
| Web order | `channel.stock_reserved` | `webshopcraft` |

Stockcraft applies `invoice.issued` and `bom.exported` only. It rejects the shop events, so warehouse quantity does not move.

To order new stock, use stockcraft: set Meldebestand on the SKU, run **Reorder plan**, then receive goods with `stock_receive`. Shopcraft does not buy from suppliers. The wholesale page sells to trade accounts.

## Layout

| Layer | Crate | Role |
|---|---|---|
| L1 | `crates/shopcraft-core` | Shared suite plus one module per mode |
| L2 | `crates/shopcraft-mcp` | `shopcraft` CLI (`--mode`) and MCP stdio |
| L3 | `crates/shopcraft-app` | egui desks. Shop menu switches mode and opens the webshop. |

## Verification

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## License

MIT OR Apache-2.0
