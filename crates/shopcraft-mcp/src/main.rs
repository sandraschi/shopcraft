use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use craft_biz::mcp::{serve_stdio, ToolHost};
use serde_json::{json, Value};
use shopcraft_core::models::{CheckoutInput, CheckoutLineInput};
use shopcraft_core::suite::ShopSuite;

#[derive(Parser)]
#[command(
    name = "shopcraft",
    version,
    about = "Shopcraft storefronts: retail, web, amazon, subscription, digital, wholesale, ticket, rental"
)]
struct Cli {
    /// Directory of per-mode SQLite files
    #[arg(long, default_value = "shopcraft-data")]
    db: PathBuf,

    /// Storefront mode for status, seed, tool, and retail commands
    #[arg(long, default_value = "retail")]
    mode: String,

    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run MCP 2024-11-05 stdio server for every mode
    Mcp,
    /// Show the selected mode status
    Status,
    /// Seed demo data for the selected mode
    Seed,
    /// Call one mode-prefixed tool with a JSON object
    Tool {
        name: String,
        #[arg(long, default_value = "{}")]
        args: String,
    },
    /// List active retail POS items
    Catalog {
        #[arg(long)]
        category: Option<String>,
    },
    /// Open a retail register shift
    ShiftOpen {
        #[arg(long)]
        clerk: String,
        #[arg(long, default_value_t = 10000)]
        float_cents: i64,
    },
    /// Complete a retail POS checkout (--item SKU_OR_EAN:QTY)
    Checkout {
        #[arg(long)]
        shift: String,
        #[arg(long = "item", required = true)]
        items: Vec<String>,
        #[arg(long, default_value_t = 0)]
        cash_cents: i64,
        #[arg(long, default_value_t = 0)]
        card_cents: i64,
        #[arg(long, default_value_t = 0)]
        voucher_cents: i64,
        #[arg(long)]
        discount_bps: Option<i64>,
    },
    /// Storno a retail receipt
    Storno { receipt_no: String },
    /// Close a retail shift and issue a Z-Bon
    ShiftClose {
        #[arg(long)]
        shift: String,
        #[arg(long)]
        counted_cash_cents: i64,
    },
    /// Render a retail 42-col thermal receipt preview
    Receipt { receipt_no: String },
}

fn parse_line(spec: &str) -> Result<CheckoutLineInput, String> {
    let (code, qty_str) = spec
        .split_once(':')
        .ok_or_else(|| format!("expected CODE:QTY, got '{spec}'"))?;
    let qty: i64 = qty_str
        .parse()
        .map_err(|_| format!("invalid qty in '{spec}'"))?;
    Ok(CheckoutLineInput {
        code: code.to_string(),
        qty,
    })
}

fn require_retail(mode: &str, command: &str) -> Result<(), String> {
    if mode != "retail" {
        return Err(format!(
            "{command} belongs to --mode retail. Use `status`, `seed`, or `tool` for {mode}."
        ));
    }
    Ok(())
}

fn print_value(value: &Value, json_only: bool) -> Result<(), String> {
    let text = if json_only {
        serde_json::to_string(value).map_err(|e| e.to_string())?
    } else {
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    };
    println!("{text}");
    Ok(())
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let mut suite = ShopSuite::open(&cli.db)?;

    if let Command::Mcp = cli.cmd {
        serve_stdio(&mut suite).map_err(|e| e.to_string())?;
        return Ok(());
    }

    if let Command::Receipt { receipt_no } = &cli.cmd {
        require_retail(&cli.mode, "receipt")?;
        if !cli.json {
            let text = suite
                .retail
                .store
                .format_receipt_text(receipt_no)
                .map_err(|e| e.to_string())?;
            print!("{text}");
            return Ok(());
        }
    }

    let result: Value = match cli.cmd {
        Command::Mcp => unreachable!(),
        Command::Status => {
            let tool = ShopSuite::status_tool(&cli.mode)?;
            suite.call(tool, &json!({}))?
        }
        Command::Seed => {
            let tool = ShopSuite::seed_tool(&cli.mode)?;
            suite.call(tool, &json!({}))?
        }
        Command::Tool { name, args } => {
            if !ShopSuite::tool_matches_mode(&cli.mode, &name) {
                return Err(format!("tool '{name}' is not in mode '{}'", cli.mode));
            }
            let parsed: Value = serde_json::from_str(&args).map_err(|e| e.to_string())?;
            suite.call(&name, &parsed)?
        }
        Command::Catalog { category } => {
            require_retail(&cli.mode, "catalog")?;
            suite.call("shop_catalog_list", &json!({ "category": category }))?
        }
        Command::ShiftOpen { clerk, float_cents } => {
            require_retail(&cli.mode, "shift-open")?;
            suite.call(
                "shop_shift_open",
                &json!({ "clerk_name": clerk, "opening_float_cents": float_cents }),
            )?
        }
        Command::Checkout {
            shift,
            items,
            cash_cents,
            card_cents,
            voucher_cents,
            discount_bps,
        } => {
            require_retail(&cli.mode, "checkout")?;
            let parsed = items
                .iter()
                .map(|s| parse_line(s))
                .collect::<Result<Vec<_>, _>>()?;
            let input = CheckoutInput {
                shift_id: shift,
                items: parsed,
                discount_bps,
                cash_cents,
                card_cents,
                voucher_cents,
            };
            let val = serde_json::to_value(input).map_err(|e| e.to_string())?;
            suite.call("shop_checkout", &val)?
        }
        Command::Storno { receipt_no } => {
            require_retail(&cli.mode, "storno")?;
            suite.call("shop_storno", &json!({ "receipt_no": receipt_no }))?
        }
        Command::ShiftClose {
            shift,
            counted_cash_cents,
        } => {
            require_retail(&cli.mode, "shift-close")?;
            suite.call(
                "shop_shift_close",
                &json!({ "shift_id": shift, "counted_cash_cents": counted_cash_cents }),
            )?
        }
        Command::Receipt { receipt_no } => {
            require_retail(&cli.mode, "receipt")?;
            suite.call("shop_receipt_print", &json!({ "receipt_no": receipt_no }))?
        }
    };

    print_value(&result, cli.json)
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
