use crate::{Db, Deconstruct, Fabricate, Item, LevelResource, Localization, Prices, RequiredItem};

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub(crate) fn parse_db(game_path: &Path) -> Db {
    let version = parse_version(game_path);
    let localization = parse_localization(game_path);
    let items = parse_items(game_path, &localization);
    Db {
        version,
        items,
        localization,
    }
}

pub(crate) fn parse_version(game_path: &Path) -> String {
    let path = game_path.join("Barotrauma.deps.json");
    let content = std::fs::read_to_string(path).unwrap();
    let re = regex::Regex::new(r#""Barotrauma/([^"]+)""#).unwrap();
    for cap in re.captures_iter(&content) {
        return cap[1].to_string();
    }
    panic!("this should never happen - couldn't extract version");
}

fn parse_localization(game_path: &Path) -> Localization {
    let loc_path = game_path
        .join("Content")
        .join("Texts")
        .join("English")
        .join("EnglishVanilla.xml");
    let content = std::fs::read_to_string(loc_path).unwrap();
    let doc = roxmltree::Document::parse(&content).unwrap();
    let root = doc.root();

    let infotexts_elem = root
        .children()
        .find(|x| x.tag_name().name() == "infotexts")
        .unwrap();

    let mut localization = Localization {
        entries: HashMap::new(),
    };

    for item in infotexts_elem.children().filter(|x| x.is_element()) {
        let tag = item.tag_name().name();
        let content = match item.children().find(|x| x.is_text()) {
            Some(x) => x,
            None => continue,
        }
        .text()
        .unwrap();
        localization
            .entries
            .insert(tag.to_string(), content.to_string());
    }

    localization
}

fn parse_bool(s: &str) -> bool {
    match s {
        "true" => true,
        "false" => false,
        _ => panic!(),
    }
}

fn parse_prices(elem: roxmltree::Node) -> Prices {
    let mut inner = BTreeMap::new();
    let base_price = elem.attribute("baseprice").unwrap().parse::<i32>().unwrap();

    // Note: e.g. wrench and diving knife are lacking both "soldeverywhere" and "sold", yet they are common
    let is_sold_everywhere = elem.attribute("soldeverywhere").map(|x| parse_bool(x));

    for child in elem.children().filter(|x| x.tag_name().name() == "Price") {
        for attr in child.attributes() {
            if !["locationtype", "multiplier", "sold", "minavailable"].contains(&attr.name()) {
                println!("{:?}", attr);
            }
        }
        let has_min = child.attribute("minavailable").is_some();
        let mut is_sold = child.attribute("sold").map(|x| parse_bool(x));
        if has_min || is_sold_everywhere == Some(true) {
            assert!(is_sold != Some(false));
            is_sold = Some(true);
        }
        // if neither of the "sold" "minavailable", "soldeverywhere" are present, then the item *is* sold: see, e.g. wrenches
        if !has_min && is_sold_everywhere == None && is_sold == None {
            is_sold = Some(true);
        }
        let is_sold = is_sold.unwrap();

        let multiplier = child
            .attribute("multiplier")
            .unwrap_or("1.0")
            .parse()
            .unwrap();
        inner.insert(
            child.attribute("locationtype").unwrap().to_string(),
            (multiplier, is_sold),
        );
    }
    Prices {
        base_price,
        locations: inner,
    }
}

// this is like O(n^2) or something but i don't care
fn dedup_things<T: Clone + Eq>(things: &[T]) -> Vec<(T, i32)> {
    let mut dedupped: Vec<(T, i32)> = vec![];
    for thing in things.iter() {
        if dedupped.iter().any(|(d, _)| d == thing) {
            continue;
        }
        let count = things.iter().filter(|t| t == &thing).count();
        dedupped.push((thing.clone(), count as i32));
    }
    dedupped
}

fn parse_fabricate(elem: roxmltree::Node) -> Fabricate {
    let mats = elem
        .children()
        .filter(|x| x.tag_name().name() == "RequiredItem" || x.tag_name().name() == "Item")
        .map(|e| {
            if let Some(id) = e.attribute("identifier") {
                RequiredItem::Id(id.to_string())
            } else if let Some(tag) = e.attribute("tag") {
                RequiredItem::Tag(tag.to_string())
            } else {
                panic!()
            }
        })
        .collect::<Vec<_>>();

    let mats = dedup_things(&mats);
    // let mats = vec![];
    // they are NOT sorted in the UI

    let skills = elem
        .children()
        .filter(|x| x.tag_name().name() == "RequiredSkill")
        .map(|x| {
            (
                x.attribute("identifier").unwrap().to_string(),
                x.attribute("level").unwrap().parse::<i32>().unwrap(),
            )
        })
        .collect();
    let time = elem
        .attribute("requiredtime")
        .unwrap_or("0")
        .parse::<i32>()
        .unwrap();
    let out_amount = elem
        .attribute("amount")
        .unwrap_or("1")
        .parse::<i32>()
        .unwrap();
    Fabricate {
        out_amount,
        time,
        skills,
        mats,
        fabricator: elem.attribute("suitablefabricators").unwrap().to_string(),
    }
}

fn parse_deconstruct(elem: roxmltree::Node) -> Deconstruct {
    let mats = elem
        .children()
        .filter(|x| x.tag_name().name() == "Item" || x.tag_name().name() == "RequiredItem")
        .map(|e| e.attribute("identifier").unwrap().to_string())
        .collect::<Vec<_>>();

    let time = elem.attribute("time").unwrap().parse::<i32>().unwrap();

    let mats = dedup_things(&mats);

    Deconstruct { time, mats }
}

fn parse_level_resource(elem: roxmltree::Node) -> LevelResource {
    let mut comonness_default = None;
    let mut comonnesses = HashMap::new();
    for item in elem
        .children()
        .filter(|x| x.tag_name().name() == "Commonness")
    {
        let com = item
            .attribute("commonness")
            .unwrap()
            .parse()
            .unwrap();
        let leveltype = item.attribute("leveltype");
        match leveltype {
            Some(lt) => {
                comonnesses.insert(lt.to_string(), com);
            }
            None => {
                assert_eq!(comonness_default, None);
                comonness_default = Some(com);
            }
        };
    }
    LevelResource {
        comonness_default: comonness_default.unwrap(),
        comonness: comonnesses,
    }
}

pub(crate) fn parse_items(game_path: impl AsRef<Path>, localization: &Localization) -> Vec<Item> {
    let game_path = game_path.as_ref();

    let items_path = game_path.join("Content").join("Items");
    let mut items: Vec<Item> = vec![];
    for entry in walkdir::WalkDir::new(items_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext.to_string_lossy() == "xml")
                .unwrap_or(false)
                && entry.path().file_name().unwrap().to_string_lossy() != "uniqueitems.xml"
        })
    {
        // log::debug!("{}", entry.path().display());
        let content = std::fs::read_to_string(entry.path()).unwrap();
        let doc = roxmltree::Document::parse(&content).unwrap();
        let item_container_elem = match doc
            .root()
            .children()
            .filter(|elem| elem.tag_name().name() == "Items")
            .next()
        {
            Some(x) => x,
            None => continue,
        };
        for item_elem in item_container_elem
            .children()
            .filter(|elem| elem.is_element())
        {
            // log::debug!("{:?}", item_elem.attribute("identifier"));
            let price_elem = item_elem
                .children()
                .filter(|p| p.tag_name().name() == "Price")
                .next();
            let price_elem = match price_elem {
                Some(p) => p,
                None => continue,
            };
            let fabricate_elem = item_elem
                .children()
                .filter(|p| p.tag_name().name() == "Fabricate")
                .next();
            let deconstruct_elem = item_elem
                .children()
                .filter(|p| p.tag_name().name() == "Deconstruct")
                .next();

            let id = item_elem.attribute("identifier").unwrap().to_string();
            let mut name: Option<String> = item_elem.attribute("name").map(|x| x.to_string());
            if name.as_deref() == Some("") {
                name = None;
            }
            if name.is_none() {
                name = item_elem
                    .attribute("nameidentifier")
                    .and_then(|nid| localization.item_name_opt(nid))
                    .or_else(|| localization.item_name_opt(id.as_str()))
                    .map(|x| x.to_string());
            };
            let level_resource = item_elem
                .children()
                .find(|x| x.tag_name().name() == "LevelResource")
                .map(|elem| parse_level_resource(elem));

            let item = Item {
                name,
                id,
                tags: item_elem
                    .attribute("Tags")
                    .unwrap_or("")
                    .split(",")
                    .map(|x| x.to_string())
                    .collect(),
                prices: parse_prices(price_elem),
                fabricate: fabricate_elem.map(|e| parse_fabricate(e)),
                deconstruct: deconstruct_elem.map(|e| parse_deconstruct(e)),
                has_inventory_icon: item_elem
                    .children()
                    .find(|x| x.tag_name().name() == "InventoryIcon")
                    .is_some(),
                has_sprite: item_elem
                    .children()
                    .find(|x| x.tag_name().name() == "Sprite")
                    .is_some(),
                level_resource,
            };

            items.push(item)
        }
    }
    items
}
