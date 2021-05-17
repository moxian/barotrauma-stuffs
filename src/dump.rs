use crate::{Db, Item, RequiredItem};

use std::io::Write;
use std::path::Path;

pub(crate) fn dump_prices(items: &[Item]) {
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
        .flat_map(|item| item.prices.locations.keys())
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
                item.prices
                    .locations
                    .get(loc.as_str())
                    .map(|x| x.0)
                    .unwrap_or(1.0)
            ));
        }
        writer.write_record(record).unwrap();
    }
}

// There are two kinds of sprites: in the world, and in the inventory.
// Wiki is inconsisntent in whether ItemName.png represents the former or the latter
// So sometimes we need to postfix the filename with _icon to get the in-inventory look.
// This function denotes the several exeptions that do require such postfixing.
fn is_iconic_item(item: &Item) -> bool {
    return false;
}

fn linkify_item(items: &[Item], id: &str, cnt: i32, size: Option<i32>) -> String {
    let item = items.iter().find(|it| &it.id == id).unwrap();
    let name = item.name.as_deref().unwrap();
    let mut line = if let Some(size) = size {
        format!(
            "{{{{Hyperlink|{name}|{size}px}}}}",
            name = name,
            size = size
        )
    } else {
        format!("{{{{Hyperlink|{name}}}}}", name = name)
    };
    if cnt > 1 {
        line += &format!(" (x{})", cnt);
    }
    line
}

pub(crate) fn dump_fabricate(items: &[Item], fab_type: &str) -> std::io::Result<()> {
    let out_path = Path::new(&format!("out/fabricate_{}.txt", fab_type)).to_owned();
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
! style="width: 30%" | <abbr title="If different from the crafting recipe">Deconstructs to</abbr>
"#
        .as_bytes(),
    )?;

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

    let make_item_line = |item: &Item, name_override: Option<&str>| {
        // debug!("{:?}", item.id);
        let fabricate = item.fabricate.as_ref().unwrap();
        let fabricate_mat_names = fabricate
            .mats
            .iter()
            .map(|(m, cnt)| match m {
                RequiredItem::Id(id) => linkify_item(&items, id, *cnt, Some(30)),
                RequiredItem::Tag(tag) => match tag.as_str() {
                    "wire" => {
                        // "{{{{Hyperlink| Wire|30px|}}}} (any)".to_string(),
                        linkify_item(&items, "wire", *cnt, Some(30))
                    }
                    _ => panic!("{:?}", tag),
                },
            })
            .collect::<Vec<_>>();
        let fabricate_line = fabricate_mat_names.join(" <br> ");

        let decon_line = match item.deconstruct.as_ref() {
            None => "Not deconstructable".to_owned(),
            Some(d) => {
                let mut fabricate_mat_ids = fabricate
                    .mats
                    .iter()
                    .map(|(m, cnt)| match m {
                        RequiredItem::Id(id) => (id.to_string(), *cnt),
                        RequiredItem::Tag(_) => ("not_found".into(), 99),
                    })
                    .collect::<Vec<_>>();
                fabricate_mat_ids.sort();
                let mut d_mats = d.mats.clone();
                d_mats.sort();
                if fabricate_mat_ids == d_mats {
                    "-".to_owned()
                } else {
                    d.mats
                        .iter()
                        .map(|(mat_id, cnt)| linkify_item(&items, mat_id, *cnt, Some(30)))
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
            let pic_name = if is_iconic_item(&item) {
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
        if fabricate.fabricator != fab_type {
            continue;
        };

        let line = make_item_line(&item, None);
        file.write(line.as_bytes())?;
    }

    for (gc_tags, gc_exceptions, gc_name) in grouped_category_tags {
        let mut canonical_line = None;
        for item in &items {
            if gc_exceptions.contains(&item.id.as_str()) {
                continue;
            }
            if let Some(f) = item.fabricate.as_ref() {
                if f.fabricator != fab_type {
                    continue;
                }
            } else {
                continue;
            }
            if !item
                .tags
                .iter()
                .any(|item_tag| gc_tags.contains(&item_tag.as_str()))
            {
                continue;
            }

            let mut fake_item = item.clone();
            if let Some(fab) = fake_item.fabricate.as_mut() {
                fab.mats.sort();
            }

            let this_line = make_item_line(&fake_item, Some(gc_name));
            if let Some(cl) = canonical_line.as_ref() {
                assert_eq!(&this_line, cl)
            } else {
                canonical_line = Some(this_line);
            }
        }
        if let Some(cl) = canonical_line {
            file.write_all(cl.as_bytes())?;
        } else {
            // wrong fab type likely
        }
    }

    file.write_all(
        r#"|-
|}
"#
        .as_bytes(),
    )?;
    Ok(())
}

// TERRIBLE TERRIBLE COPY-PASTE
// but hopefully this is readable than even more ifs?..
pub(crate) fn dump_deconstruct(items: &[Item]) -> std::io::Result<()> {
    let out_path = Path::new("out/fabricate_deconstruct.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)
        .unwrap();

    file.write(
        r#"{| class="wikitable sortable" style="width: 30%; font-size: 90%;"
! style="width: 40%" | Item
! style="width: 20%" | Time (seconds)
! style="width: 60%" | Deconstructs to
"#
        .as_bytes(),
    )?;

    let mut items = items.to_vec();
    items.sort_by_key(|i| i.name.clone());

    let blacklist = vec!["wire", "psilotoadegg", "balloonegg", "orangeboyegg"];

    let make_item_line = |item: &Item, name_override: Option<&str>| {
        // debug!("{:?}", item.id);
        let decon = item.deconstruct.as_ref().unwrap();

        let decon_line = decon
            .mats
            .iter()
            .map(|(mat_id, cnt)| linkify_item(&items, mat_id, *cnt, Some(30)))
            .collect::<Vec<_>>()
            .join(" <br> ");

        let display_name = if let Some(no) = name_override {
            no.to_string()
        } else {
            let item_name = item.name.as_ref().unwrap().as_str();
            let pic_name = if is_iconic_item(&item) {
                format!("{}_icon", item_name)
            } else if item.id == "smallmudraptoregg" {
                // AAAAAA
                "Mudraptor_Egg_Small".into()
            } else if item.id == "peanutegg" {
                "Strange Eggs".into()
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
| align="center" | {display_name}
| align="center" | {time}
| align="left-index" | {deconstruct}
"#,
            display_name = display_name,
            time = decon.time,
            deconstruct = decon_line,
        );
        line
    };

    for item in &items {
        if blacklist.contains(&item.id.as_str()) {
            continue;
        }
        let decon = match &item.deconstruct {
            None => continue,
            Some(f) => f,
        };
        if item.fabricate.is_some() {
            continue; // only show non-constructible stuffs
        }
        if decon.mats.is_empty() {
            continue;
        }

        let line = make_item_line(&item, None);
        file.write_all(line.as_bytes())?;
    }

    file.write_all(
        r#"|-
|}
"#
        .as_bytes(),
    )?;
    Ok(())
}

fn format_mineral(item: &Item) -> String {
    let mut fields: Vec<(String, String)> = vec![];
    // fields.("{{Gatherable Materials
    fields.push(("name".into(), item.name.as_ref().unwrap().to_string()));
    if item.tags.contains(&"ore".to_string()) {
        fields.push(("kind".into(), "mineral".into()));
    } else {
        panic!();
    }
    let lr = item.level_resource.as_ref().unwrap();
    if lr.comonness.len() == 0 {
        fields.push(("comonness".into(), lr.comonness_default.to_string()))
    } else {
        for (level, biome) in &[
            ("coldcaverns", "coldcaverns"),
            ("ridgebasic", "europanridge"),
            ("plateaubasic", "theaphoticplateau"),
            ("greatseabasic", "thegreatsea"),
            ("wastesbasic", "hydrothermalwastes"),
        ] {
            let com = if level != &"coldcaverns" {
                lr.comonness.get(*level).unwrap()
            } else {
                lr.comonness.get(*level).unwrap_or(&lr.comonness_default)
            };
            fields.push((format!("comonness_{}", biome), (com * 100.0).round().to_string()));
        }
    }

    let mut result = String::new();
    result.push_str("{{Gatherable Materials\n");
    result.push_str(
        &fields
            .iter()
            .map(|(k, v)| format!("| {} = {}", k, v))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    result.push_str("\n}}");
    result
}

fn format_infobox(item: &Item, db: &Db) -> Option<String> {
    let name = item.name.clone().unwrap();
    let mut category = None;
    if item.tags.contains(&"ore".into()) {
        category = Some("ore");
    }
    let category = category?;

    let mut fields: Vec<(String, String)> = vec![];
    fields.push(("identifier".into(), item.id.clone()));
    fields.push(("name".into(), name.clone()));
    if category == "ore" {
        fields.push(("image".into(), format!("{}.png", name)));
        // fields.push(("caption".into(), "Mined mineral".into()));
        fields.push((
            "caption".into(),
            format!("''{}''", db.localization.item_description(&item.id)),
        ));
        fields.push(("image2".into(), format!("{}_Mineral.png", name)));
        fields.push(("caption2".into(), "Sprite in the environment".into()));
        fields.push(("icon".into(), format!("{}.png", name)));
        fields.push(("sprite".into(), format!("{}_Mineral.png", name)));
    } else {
        fields.push(("image".into(), format!("{}.png", name)));
        fields.push(("caption".into(), "Inventory icon".into()));
        fields.push(("image2".into(), format!("{}_sprite.png", name)));
        fields.push(("caption2".into(), "Sprite".into()));
        fields.push(("icon".into(), format!("{}.png", name)));
        fields.push(("sprite".into(), format!("{}_sprite.png", name)));
    }

    //
    // fields.push(("type".into(), category.into()));
    //
    fields.push(("baseprice".into(), format!("{}", item.prices.base_price)));

    let is_sold_anywhere = item.prices.locations.values().any(|(_, is_sold)| *is_sold);
    if !is_sold_anywhere {
        fields.push(("unbuyable".into(), "true".into()));
    }
    for loc in &["outpost", "city", "research", "military", "mine"] {
        let (mult, is_sold_here) = item.prices.locations.get(&loc.to_string()).unwrap();
        fields.push((format!("{}multiplier", loc), format!("{}", mult)));
        if is_sold_anywhere && !is_sold_here {
            fields.push((format!("{}unbuyable", loc), "true".into()));
        }
    }

    if category == "ore" {
        fields.push(("noreq".into(), "Yes".into()));
    } else {
        panic!();
    }

    //
    if let Some(fab) = item.fabricate.as_ref() {
        fields.push(("fabricator".into(), "Yes".to_string())); // sadly required
        fields.push(("fabricatedamount".into(), fab.out_amount.to_string()));
        fields.push(("fabricatortime".into(), fab.time.to_string()));
        // TODO: only first skill is used. Others are ignored (only relevant for health scanner?)
        assert!(fab.skills.len() <= 1);
        if let Some((skill, level)) = fab.skills.get(0) {
            fields.push(("fabricatorskill".into(), skill.to_string()));
            fields.push(("fabricatorskilllevel".into(), level.to_string()));
        }
        assert_eq!(fab.fabricator, "fabricator");
        let mats = fab
            .mats
            .iter()
            .map(|(mat, cnt)| match mat {
                RequiredItem::Id(id) => linkify_item(&db.items, id, *cnt, None),
                RequiredItem::Tag(tag) => match tag.as_str() {
                    "wire" => linkify_item(&db.items, "wire", *cnt, None),
                    _ => panic!("{:?}", tag),
                },
            })
            .collect::<Vec<_>>()
            .join("\n");
        fields.push(("fabricatormaterials".into(), mats));
    }

    if let Some(decon) = item.deconstruct.as_ref() {
        fields.push(("deconstructor".into(), "Yes".to_string())); // sadly required
        fields.push(("deconstructortime".into(), decon.time.to_string()));
        let mats = decon
            .mats
            .iter()
            .map(|(mat_id, cnt)| linkify_item(&db.items, mat_id, *cnt, None))
            .collect::<Vec<_>>()
            .join("\n");
        fields.push(("deconstructormaterials".into(), mats));
    }

    let mut result = String::new();
    if category == "ore" {
        result += "{{Main|Minerals}}\n\n";
        result += &format!("{{{{Version|{}}}}}\n", db.version);
    }
    result += &"{{Items infobox";
    for (k, v) in fields {
        result += &format!("\n| {} = {}", k, v);
    }
    result += "\n}}";
    if category == "ore" {
        result += "\n";
        result += &format_mineral(item);
    }
    Some(result)
}

pub(crate) fn dump_infoboxes(db: &Db) {
    let out_path = Path::new("out/infoboxes.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out_path)
        .unwrap();
    for item in &db.items {
        let name = match item.name.as_ref() {
            Some(n) => n,
            None => continue,
        };
        let ib = match format_infobox(item, &db) {
            Some(x) => x,
            None => continue,
        };

        file.write_all(format!("\n\n ===  {}  ===  \n\n", name).as_bytes())
            .unwrap();
        file.write_all(ib.as_bytes()).unwrap();
    }
}
