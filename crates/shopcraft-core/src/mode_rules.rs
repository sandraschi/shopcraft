#![allow(clippy::expect_used)]

use chrono::NaiveDate;

use crate::models::{CheckoutInput, CheckoutLineInput};
use crate::modes::amazon::AmazonError;
use crate::modes::digital::DigitalOrderInput;
use crate::modes::rental::{CreateBookingInput, RentalError};
use crate::modes::subscription::SubscribeInput;
use crate::modes::ticket::{IssueTicketInput, TicketError};
use crate::modes::web::{CreateOrderInput, OrderItemInput, ShopError};
use crate::modes::wholesale::WholesaleError;
use crate::store::PosStore;

#[test]
fn retail_rejects_short_tender() {
    let mut store = PosStore::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let shift = store.open_shift("Lukas", 0).expect("shift");
    let err = store.checkout(&CheckoutInput {
        shift_id: shift.shift_id,
        items: vec![CheckoutLineInput {
            code: "CER-VASE-01".into(),
            qty: 1,
        }],
        discount_bps: None,
        cash_cents: 1,
        card_cents: 0,
        voucher_cents: 0,
    });
    assert!(matches!(
        err,
        Err(crate::error::PosError::InsufficientTender { .. })
    ));
}

#[test]
fn retail_discount_reduces_total_and_stock() {
    let mut store = PosStore::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let before = store
        .list_items(None)
        .expect("list")
        .into_iter()
        .find(|i| i.sku == "CER-VASE-01")
        .expect("vase");
    let shift = store.open_shift("Lukas", 0).expect("shift");
    let receipt = store
        .checkout(&CheckoutInput {
            shift_id: shift.shift_id,
            items: vec![CheckoutLineInput {
                code: "CER-VASE-01".into(),
                qty: 1,
            }],
            discount_bps: Some(1000),
            cash_cents: 0,
            card_cents: before.price_cents,
            voucher_cents: 0,
        })
        .expect("checkout");
    assert_eq!(
        receipt.total_cents,
        before.price_cents - before.price_cents / 10
    );
    let after = store
        .list_items(None)
        .expect("list")
        .into_iter()
        .find(|i| i.sku == "CER-VASE-01")
        .expect("vase");
    assert_eq!(after.stock_qty, before.stock_qty - 1);
}

#[test]
fn web_rejects_an_empty_cart() {
    let mut store = crate::modes::web::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.create_order(&CreateOrderInput {
        customer_name: "Clara Novak".into(),
        customer_email: "clara@example.at".into(),
        shipping_address: "Neubaugasse 12".into(),
        zone_code: "AT".into(),
        promo_code: None,
        items: vec![],
    });
    assert!(matches!(err, Err(ShopError::Invalid(_))));
}

#[test]
fn web_unknown_shipping_zone_is_not_found() {
    let mut store = crate::modes::web::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.create_order(&CreateOrderInput {
        customer_name: "Clara Novak".into(),
        customer_email: "clara@example.at".into(),
        shipping_address: "Neubaugasse 12".into(),
        zone_code: "ZZ".into(),
        promo_code: None,
        items: vec![OrderItemInput {
            sku: "CER-VASE-01".into(),
            qty: 1,
        }],
    });
    assert!(matches!(err, Err(ShopError::NotFound(_))));
}

#[test]
fn amazon_reprice_clamps_to_floor() {
    let store = crate::modes::amazon::AmazonStore::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let priced = store.reprice("B0ATELIER1", 1000).expect("reprice");
    assert!(priced.clamped_to_floor);
    assert_eq!(priced.new_price_cents, 5900);
    assert!(!priced.buy_box_won);
}

#[test]
fn amazon_rejects_non_positive_competitor_price() {
    let store = crate::modes::amazon::AmazonStore::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.reprice("B0ATELIER1", 0);
    assert!(matches!(err, Err(AmazonError::Invalid(_))));
}

#[test]
fn subscription_renewal_advances_one_cadence() {
    let mut store = crate::modes::subscription::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let sub = store
        .subscribe(&SubscribeInput {
            customer_name: "Ada Berger".into(),
            customer_email: "ada@example.at".into(),
            iban: "AT611904300234573201".into(),
            plan_sku: "SUB-COFFEE-01".into(),
        })
        .expect("subscribe");
    assert_eq!(sub.cadence_days, 14);
    let billed = store.bill_renewal(&sub.sub_number).expect("bill");
    let start = NaiveDate::parse_from_str(&sub.next_billing_date, "%Y-%m-%d").expect("start");
    let end = NaiveDate::parse_from_str(&billed.next_billing_date, "%Y-%m-%d").expect("end");
    assert_eq!((end - start).num_days(), 14);
    assert_eq!(billed.cycles_billed, sub.cycles_billed + 1);
}

#[test]
fn subscription_rejects_a_short_iban() {
    let mut store = crate::modes::subscription::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.subscribe(&SubscribeInput {
        customer_name: "Ada Berger".into(),
        customer_email: "ada@example.at".into(),
        iban: "AT12".into(),
        plan_sku: "SUB-COFFEE-01".into(),
    });
    assert!(err.is_err());
}

#[test]
fn digital_license_keys_are_unique_per_order() {
    let mut store = crate::modes::digital::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let first = store
        .checkout(&DigitalOrderInput {
            buyer_name: "Nora".into(),
            buyer_email: "nora@example.at".into(),
            country_code: "AT".into(),
            items: vec!["DIG-RUST-PRO".into()],
        })
        .expect("first");
    let second = store
        .checkout(&DigitalOrderInput {
            buyer_name: "Nora".into(),
            buyer_email: "nora@example.at".into(),
            country_code: "AT".into(),
            items: vec!["DIG-RUST-PRO".into()],
        })
        .expect("second");
    assert_ne!(
        first.licenses[0].license_key,
        second.licenses[0].license_key
    );
    assert!(first.licenses[0].license_key.starts_with("DSC-"));
}

#[test]
fn digital_checkout_uses_the_buyer_country_rate() {
    let mut store = crate::modes::digital::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let at = store
        .checkout(&DigitalOrderInput {
            buyer_name: "Nora".into(),
            buyer_email: "nora-at@example.at".into(),
            country_code: "AT".into(),
            items: vec!["DIG-RUST-PRO".into()],
        })
        .expect("at");
    let de = store
        .checkout(&DigitalOrderInput {
            buyer_name: "Nora".into(),
            buyer_email: "nora-de@example.de".into(),
            country_code: "DE".into(),
            items: vec!["DIG-RUST-PRO".into()],
        })
        .expect("de");
    assert_eq!(at.vat_pct, 20);
    assert_eq!(de.vat_pct, 19);
    assert!(at.vat_cents > de.vat_cents);
}

#[test]
fn wholesale_quote_requires_a_line() {
    let store = crate::modes::wholesale::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.quote("B2B-1001", &[]);
    assert!(matches!(err, Err(WholesaleError::Invalid(_))));
}

#[test]
fn ticket_oversell_is_sold_out() {
    let store = crate::modes::ticket::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.issue_ticket(&IssueTicketInput {
        attendee_name: "Max".into(),
        attendee_email: "max@example.at".into(),
        code: "EVT-CERAMIC".into(),
        quantity: 15,
    });
    assert!(matches!(err, Err(TicketError::SoldOut { .. })));
}

#[test]
fn ticket_rejects_zero_quantity() {
    let store = crate::modes::ticket::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.issue_ticket(&IssueTicketInput {
        attendee_name: "Max".into(),
        attendee_email: "max@example.at".into(),
        code: "EVT-CERAMIC".into(),
        quantity: 0,
    });
    assert!(matches!(err, Err(TicketError::InvalidQuantity(0))));
}

#[test]
fn rental_rejects_a_reversed_date_range() {
    let mut store = crate::modes::rental::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    let err = store.book(&CreateBookingInput {
        customer_name: "Max Hofer".into(),
        customer_email: "max@example.at".into(),
        sku: "RNT-DRONE-05".into(),
        start_date: "2026-11-22".into(),
        end_date: "2026-11-20".into(),
    });
    assert!(matches!(err, Err(RentalError::Invalid(_))));
}

#[test]
fn rental_single_unit_is_unavailable_while_reserved() {
    let mut store = crate::modes::rental::Store::open_in_memory().expect("store");
    store.seed_demo().expect("seed");
    store
        .book(&CreateBookingInput {
            customer_name: "Max Hofer".into(),
            customer_email: "max@example.at".into(),
            sku: "RNT-DRONE-05".into(),
            start_date: "2026-12-01".into(),
            end_date: "2026-12-03".into(),
        })
        .expect("book");
    let left = store
        .available_units("RNT-DRONE-05", "2026-12-02", "2026-12-04")
        .expect("avail");
    assert_eq!(left, 0);
}
