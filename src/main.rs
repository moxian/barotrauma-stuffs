mod parse;
mod dump;

#[allow(unused_imports)]
use log::{debug, info};

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone)]
struct Item {
    id: String,
    name: Option<String>,
    tags: Vec<String>,
    prices: Prices,
    fabricate: Option<Fabricate>,
    deconstruct: Option<Deconstruct>,
    has_inventory_icon: bool,
    has_sprite: bool,
}

#[derive(Debug, Clone)]
struct Prices {
    // kind -> multiplier
    inner: BTreeMap<String, f32>,
}

#[derive(Debug, Clone)]
struct Fabricate {
    out_amount: i32,
    skills: Vec<(String, i32)>,
    time: i32,
    mats: Vec<(RequiredItem, i32)>,
    fabricator: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord)]
enum RequiredItem {
    Id(String),
    Tag(String),
}
impl std::cmp::PartialOrd for RequiredItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(match (self, other) {
            (RequiredItem::Id(a), RequiredItem::Id(b)) => a.cmp(b),
            (RequiredItem::Tag(a), RequiredItem::Tag(b)) => a.cmp(b),
            (RequiredItem::Id(_), RequiredItem::Tag(_)) => std::cmp::Ordering::Less,
            (RequiredItem::Tag(_), RequiredItem::Id(_)) => std::cmp::Ordering::Greater,
        })
    }
}

#[derive(Debug, Clone)]
struct Deconstruct {
    time: i32,
    mats: Vec<(String, i32)>,
}



fn stuff() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let game_path = Path::new(r"D:\games\SteamLibrary\steamapps\common\Barotrauma");

    let items = parse::parse_items(game_path);

    dump::dump_prices(&items);
    dump::dump_fabricate(&items, "fabricator").unwrap();
    dump::dump_fabricate(&items, "medicalfabricator").unwrap();
    dump::dump_deconstruct(&items).unwrap();
}

fn main() {
    stuff();
}
