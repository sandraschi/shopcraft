# ShopCraft

Local-first physical shop POS, cash drawer (*Kassa*), Z-Bon, and thermal receipt engine in pure Rust.

## Quick Start

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p shopcraft-mcp -- seed
cargo run -p shopcraft-mcp -- shift-open --clerk "Lukas" --float-cents 10000
```

## License

MIT OR Apache-2.0
