mod dump;
mod parse;

#[allow(unused_imports)]
use log::{debug, info};

use std::collections::{BTreeMap, HashMap};
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
    level_resource: Option<LevelResource>,
}

#[derive(Debug, Clone)]
struct LevelResource{
    comonness_default: f32,
    comonness: HashMap<String, f32>,
}

#[derive(Debug, Clone)]
struct Prices {
    base_price: i32,
    // location -> (multiplier, is_sold)
    locations: BTreeMap<String, (f32, bool)>,
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

struct Localization {
    entries: HashMap<String, String>,
}
impl Localization {
    fn item_name_opt(&self, id: &str) -> Option<&str> {
        self.entries
            .get(&format!("entityname.{}", id))
            .map(|s| s.as_str())
    }
    fn item_description(&self, id: &str) -> &str {
        self.entries
            .get(&format!("entitydescription.{}", id))
            .unwrap()
            .as_str()
    }
}

struct Db {
    version: String,
    items: Vec<Item>,
    localization: Localization,
}

fn stuff() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let game_path = Path::new(r"D:\games\SteamLibrary\steamapps\common\Barotrauma");

    let db = parse::parse_db(game_path);

    dump::dump_prices(&db.items);
    dump::dump_fabricate(&db.items, "fabricator").unwrap();
    dump::dump_fabricate(&db.items, "medicalfabricator").unwrap();
    dump::dump_deconstruct(&db.items).unwrap();

    dump::dump_infoboxes(&db);
}

fn main() {
    stuff();
}
