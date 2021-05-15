use crate::{Item, Prices, Fabricate, RequiredItem, Deconstruct};

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

struct Localization {
    item_names: HashMap<String, String>,
    skill_names: HashMap<String, String>,
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
        item_names: HashMap::new(),
        skill_names: HashMap::new(),
    };

    for item in infotexts_elem.children().filter(|x| x.is_element()) {
        let tag = item.tag_name().name();
        if tag.starts_with("entityname.") {
            let entity_id = tag.rsplit(".").next().unwrap();
            let name = item
                .children()
                .find(|x| x.is_text())
                .unwrap()
                .text()
                .unwrap();
            localization
                .item_names
                .insert(entity_id.to_string(), name.to_string());
        } else if tag.starts_with("skillname.") {
            let skill_id = tag.rsplit(".").next().unwrap();
            let name = item
                .children()
                .find(|x| x.is_text())
                .unwrap()
                .text()
                .unwrap();
            localization
                .skill_names
                .insert(skill_id.to_string(), name.to_string());
        }
    }

    localization
}

fn parse_prices(elem: roxmltree::Node) -> Prices {
    let mut inner = BTreeMap::new();
    for child in elem.children().filter(|x| x.tag_name().name() == "Price") {
        inner.insert(
            child.attribute("locationtype").unwrap().to_string(),
            child
                .attribute("multiplier")
                .unwrap_or("1.0")
                .parse()
                .unwrap(),
        );
    }
    Prices { inner }
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
    let mut mats = dedup_things(&mats);
    mats.sort();

    let time = elem.attribute("time").unwrap().parse::<i32>().unwrap();

    Deconstruct { time, mats }
}

pub(crate) fn parse_items(game_path: impl AsRef<Path>) -> Vec<Item> {
    let game_path = game_path.as_ref();
    let localization = parse_localization(game_path);

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
                    .and_then(|nid| localization.item_names.get(nid))
                    .or_else(|| localization.item_names.get(id.as_str()))
                    .map(|x| x.to_string());
            };

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
                has_inventory_icon: item_elem.children().find(|x| x.tag_name().name() == "InventoryIcon").is_some(),
                has_sprite: item_elem.children().find(|x| x.tag_name().name() == "Sprite").is_some(),
            };
            items.push(item)
        }
    }
    items
}
