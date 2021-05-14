#[allow(unused_imports)]
use log::{debug, info};

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
struct Item {
    id: String,
    name: Option<String>,
    tags: Vec<String>,
    prices: Prices,
    fabricate: Option<Fabricate>,
    deconstruct: Option<Deconstruct>,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RequiredItem {
    Id(String),
    Tag(String),
}
#[derive(Debug, Clone)]
struct Deconstruct {
    time: i32,
    mats: Vec<(String, i32)>,
}

struct Localization {
    item_names: HashMap<String, String>,
    skill_names: HashMap<String, String>,
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

fn dedup_things<T: Clone + Eq + std::hash::Hash>(things: &[T]) -> Vec<(T, i32)> {
    let mut things_deduped = HashMap::new();
    for mat in things.iter() {
        things_deduped.insert(
            mat.clone(),
            things.iter().filter(|x| x == &mat).count() as i32,
        );
    }
    let things = things_deduped.into_iter().collect::<Vec<_>>();
    things
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

    let mut mats = dedup_things(&mats);
    // they are sorted in the UI
    mats.sort_by(|(a, _), (b, _)| match (a, b) {
        (RequiredItem::Id(a), RequiredItem::Id(b)) => a.cmp(b),
        (RequiredItem::Tag(a), RequiredItem::Tag(b)) => a.cmp(b),
        (RequiredItem::Id(_), RequiredItem::Tag(_)) => std::cmp::Ordering::Less,
        (RequiredItem::Tag(_), RequiredItem::Id(_)) => std::cmp::Ordering::Greater,
    });

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

fn dump_prices(items: &[Item]) {
    let out_path = Path::new("out/items_prices.csv");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    let out_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)
        .unwrap();
    let mut writer = csv::Writer::from_writer(out_file);
    let all_locations = items
        .iter()
        .flat_map(|item| item.prices.inner.keys())
        .collect::<std::collections::BTreeSet<_>>();
    let all_locations = all_locations.into_iter().collect::<Vec<_>>();
    writer
        .write_record(std::iter::once("name").chain(all_locations.iter().map(|x| x.as_str())))
        .unwrap();
    for item in items {
        let mut record = vec![];
        record.push(item.id.to_string());
        // println!("{:?} {:?}", item, all_locations);
        for loc in &all_locations {
            record.push(format!(
                "{}",
                item.prices.inner.get(loc.as_str()).unwrap_or(&1.0)
            ));
        }
        writer.write_record(record).unwrap();
    }
}

fn dump_fabricate(items: &[Item], _localization: &Localization) -> std::io::Result<()> {
    let out_path = Path::new("out/fabricate.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)
        .unwrap();

    file.write(
        r#"{| class="wikitable sortable" style="width: 50%; font-size: 90%;"
! style="width: 15%" | Item
! style="width: 30%" | Materials to Craft 
! style="width: 10%" | Time (seconds)
! style="width: 15%" | Skill 
! style="width: 30%" | Deconstructs to
"#
        .as_bytes(),
    )?;

    let iconic_items: Vec<&str> = vec![
        "organicfiber",
        "ballisticfiber",
        "bodyarmor",
        "divingsuit",
        "divingmask",
        "explosivespear",
        "headset",
        "healthscanner",
        "incendiumgrenade",
        "stungrenade",
    ];

    let mut items = items.to_vec();
    items.sort_by_key(|i| i.name.clone());

    // categories, exceptions, name
    let grouped_category_tags: Vec<(Vec<&str>, Vec<&str>, &str)> = vec![
        (
            vec!["logic", "signal"],
            vec!["fpgacircuit"],
            "[[File:Wiring Components.png| |90px|link=Wiring Components]] <br> [[Wiring Components]]",
        ),
        (vec!["sensor"], vec![], "[[File:Detectors.png| |90px|link=Detectors]] <br> [[Detectors]]",),
        (vec!["wire"], vec![], "[[File:Wire.png| |50px|link=Wire]] <br> [[Wire]]"),
    ];
    let blacklist = vec!["lightcomponent90"];

    let linkify_item = |id: &str, cnt: i32| -> String {
        let name = items
            .iter()
            .find(|it| &it.id == id)
            .unwrap()
            .name
            .as_ref()
            .unwrap();
        let mut line = if iconic_items.contains(&id) {
            format!("{{{{Hyperlink|{name}|30px|icon}}}}", name = name)
        } else {
            format!("{{{{Hyperlink|{name}|30px}}}}", name = name)
        };
        if cnt > 1 {
            line += &format!(" (x{})", cnt);
        }
        line
    };

    let make_item_line = |item: &Item, name_override: Option<&str>| {
        debug!("{:?}", item.id);
        let fabricate = item.fabricate.as_ref().unwrap();
        let fabricate_mat_names = fabricate
            .mats
            .iter()
            .map(|(m, cnt)| match m {
                RequiredItem::Id(id) => linkify_item(id, *cnt),
                RequiredItem::Tag(tag) => match tag.as_str() {
                    "wire" => {
                        // "{{{{Hyperlink| Wire|30px|}}}} (any)".to_string(),
                        linkify_item("wire", *cnt)
                    }
                    _ => panic!("{:?}", tag),
                },
            })
            .collect::<Vec<_>>();
        let fabricate_line = fabricate_mat_names.join(" <br> ");

        let decon_line = match item.deconstruct.as_ref() {
            None => "Not deconstructable".to_owned(),
            Some(d) => {
                let fabricate_mat_ids = fabricate
                    .mats
                    .iter()
                    .map(|(m, cnt)| match m {
                        RequiredItem::Id(id) => (id.to_string(), *cnt),
                        RequiredItem::Tag(_) => ("not_found".into(), 99),
                    })
                    .collect::<Vec<_>>();
                if fabricate_mat_ids == d.mats {
                    "-".to_owned()
                } else {
                    d.mats
                        .iter()
                        .map(|(mat_id, cnt)| linkify_item(mat_id, *cnt))
                        .collect::<Vec<_>>()
                        .join(" <br> ")
                }
            }
        };
        let mut skills = fabricate
            .skills
            .iter()
            .map(|(id, level)| {
                let name = match id.as_str() {
                    "electrical" => "Electrical",
                    "helm" => "Helm",
                    "mechanical" => "Mechanical",
                    "medical" => "Medical",
                    "weapons" => "Weapons",
                    _ => panic!("{:?}", fabricate),
                };
                format!("{} {}", name, level)
            })
            .collect::<Vec<_>>()
            .join(" <br> ");
        if skills == "" {
            skills = "None".into()
        }

        let display_name = if let Some(no) = name_override {
            no.to_string()
        } else {
            let item_name = item.name.as_ref().unwrap().as_str();
            let pic_name = if iconic_items.contains(&item.id.as_str()) {
                format!("{}_icon", item_name)
            } else {
                item_name.to_string()
            };
            format!(
                "[[File:{pic_name}.png| |50px|link={name}]] <br> [[{name}]]",
                pic_name = pic_name,
                name = item_name
            )
        };

        let line = format!(
            r#"|-
| align="center" | {display_name}{maybe_amount}
| align="left-index" | {fabricate} 
| align="center" | {time}
| align="center" | {skills}
| align="left-index" | {deconstruct}
"#,
            display_name = display_name,
            maybe_amount = if fabricate.out_amount > 1 {
                format!(" (x{})", fabricate.out_amount)
            } else {
                "".to_owned()
            },
            fabricate = fabricate_line,
            deconstruct = decon_line,
            time = fabricate.time,
            skills = skills,
        );
        line
    };

    for item in &items {
        if blacklist.contains(&item.id.as_str()) {
            continue;
        };
        let fabricate = match &item.fabricate {
            None => continue,
            Some(f) => f,
        };

        if item.tags.iter().any(|item_tag| {
            grouped_category_tags
                .iter()
                .any(|ct| ct.0.contains(&item_tag.as_str()) && !ct.1.contains(&item.id.as_str()))
        }) {
            continue;
        }
        if fabricate.fabricator != "fabricator" {
            continue;
        };

        let line = make_item_line(item, None);
        file.write(line.as_bytes())?;
    }

    for (gc_tags, gc_exceptions, gc_name) in grouped_category_tags {
        let mut canonical_line = None;
        for item in &items {
            if gc_exceptions.contains(&item.id.as_str()) {
                continue;
            }
            if item.fabricate.is_none() {
                continue;
            }
            if !item
                .tags
                .iter()
                .any(|item_tag| gc_tags.contains(&item_tag.as_str()))
            {
                continue;
            }
            let this_line = make_item_line(item, Some(gc_name));
            if let Some(cl) = canonical_line.as_ref() {
                assert_eq!(&this_line, cl)
            } else {
                canonical_line = Some(this_line);
            }
        }
        file.write(canonical_line.unwrap().as_bytes())?;
    }

    file.write(
        r#"|-
|}
"#
        .as_bytes(),
    )?;
    Ok(())
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

fn stuff() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let game_path = Path::new(r"D:\games\SteamLibrary\steamapps\common\Barotrauma");

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
        })
    {
        log::debug!("{}", entry.path().display());
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
            log::debug!("{:?}", item_elem.attribute("identifier"));
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
            let name: Option<String> = item_elem
                .attribute("nameidentifier")
                .and_then(|nid| localization.item_names.get(nid))
                .or_else(|| localization.item_names.get(id.as_str()))
                .map(|x| x.to_string());

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
            };
            items.push(item)
        }
    }

    dump_prices(&items);
    dump_fabricate(&items, &localization).unwrap();
}

fn main() {
    stuff();
}
