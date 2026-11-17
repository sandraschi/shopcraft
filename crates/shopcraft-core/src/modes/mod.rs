pub mod amazon;
pub mod digital;
pub mod rental;
pub mod subscription;
pub mod ticket;
pub mod web;
pub mod wholesale;

pub const MODES: &[&str] = &[
    "retail",
    "web",
    "amazon",
    "subscription",
    "digital",
    "wholesale",
    "ticket",
    "rental",
];
