//! Publish the HTML storefront next to the live catalog and open it.

use std::path::Path;
use std::process::Command;

use super::ShopApp;

const SHOP_NAME: &str = "ATELIER NORD";
const TAGLINE: &str = "Tactile objects for quiet spaces -- crafted in small European runs.";

pub fn open(app: &ShopApp) -> Result<String, String> {
    let dir = &app.suite.web.storefront_dir;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    write(
        dir,
        "index.html",
        include_str!("../../../storefront/index.html"),
    )?;
    write(
        dir,
        "styles.css",
        include_str!("../../../storefront/styles.css"),
    )?;
    write(dir, "app.js", include_str!("../../../storefront/app.js"))?;

    let mut export = app
        .suite
        .web
        .store
        .export_storefront(SHOP_NAME, TAGLINE)
        .map_err(|e| e.to_string())?;
    for product in &mut export.products {
        if let Some(file) = photo_file(&product.sku) {
            product.image_url = Some(format!("assets/{file}"));
        }
    }
    write_photos(dir)?;
    let live = !export.products.is_empty();
    let body = serde_json::to_string(&export).map_err(|e| e.to_string())?;
    write(dir, "catalog.json", &body)?;
    write(
        dir,
        "catalog.js",
        &format!("window.SHOP_CATALOG = {body};\n"),
    )?;

    let index = dir.join("index.html");
    open_html(&index)?;
    if live {
        Ok(format!(
            "Webshop opened ({} products): {}",
            export.products.len(),
            index.display()
        ))
    } else {
        Ok(format!(
            "Webshop opened with the built-in sample. Seed the catalog to publish your products. {}",
            index.display()
        ))
    }
}

fn write(dir: &Path, name: &str, body: &str) -> Result<(), String> {
    std::fs::write(dir.join(name), body).map_err(|e| e.to_string())
}

fn photo_file(sku: &str) -> Option<&'static str> {
    match sku {
        "CER-VASE-01" => Some("cer-vase-01.jpg"),
        "LGT-LAMP-02" => Some("lgt-lamp-02.jpg"),
        "TXT-THRW-03" => Some("txt-thrw-03.jpg"),
        "WRK-TRAY-04" => Some("wrk-tray-04.jpg"),
        "CER-MUG-05" => Some("cer-mug-05.jpg"),
        "WRK-NOTE-06" => Some("wrk-note-06.jpg"),
        _ => None,
    }
}

const PHOTOS: &[(&str, &[u8])] = &[
    (
        "cer-vase-01.jpg",
        include_bytes!("../../../storefront/assets/cer-vase-01.jpg"),
    ),
    (
        "lgt-lamp-02.jpg",
        include_bytes!("../../../storefront/assets/lgt-lamp-02.jpg"),
    ),
    (
        "txt-thrw-03.jpg",
        include_bytes!("../../../storefront/assets/txt-thrw-03.jpg"),
    ),
    (
        "wrk-tray-04.jpg",
        include_bytes!("../../../storefront/assets/wrk-tray-04.jpg"),
    ),
    (
        "cer-mug-05.jpg",
        include_bytes!("../../../storefront/assets/cer-mug-05.jpg"),
    ),
    (
        "wrk-note-06.jpg",
        include_bytes!("../../../storefront/assets/wrk-note-06.jpg"),
    ),
];

fn write_photos(dir: &Path) -> Result<(), String> {
    let assets = dir.join("assets");
    std::fs::create_dir_all(&assets).map_err(|e| e.to_string())?;
    for (name, bytes) in PHOTOS {
        std::fs::write(assets.join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn open_html(path: &Path) -> Result<(), String> {
    let spec = path.display().to_string();
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &spec])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("could not open {spec}: {e}"))
}
