use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use dats::{
    base::{DatId, DatPath},
    context::DatContext,
    dat_format::DatFormat,
    formats::item_info::{ItemData, ItemEntry, ItemInfoTable, ItemTextData},
    id_mapping::DatDescriptor,
};
use serde::Serialize;

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;

struct ResourceSpec {
    name: &'static str,
    descriptor: DatDescriptor,
    primary_language: &'static str,
}

const RESOURCE_SPECS: [ResourceSpec; 6] = [
    ResourceSpec {
        name: "general_items",
        descriptor: DatDescriptor::GeneralItems,
        primary_language: "english",
    },
    ResourceSpec {
        name: "usable_items",
        descriptor: DatDescriptor::UsableItems,
        primary_language: "english",
    },
    ResourceSpec {
        name: "weapons",
        descriptor: DatDescriptor::Weapons,
        primary_language: "english",
    },
    ResourceSpec {
        name: "armor",
        descriptor: DatDescriptor::Armor,
        primary_language: "english",
    },
    ResourceSpec {
        name: "puppet_items",
        descriptor: DatDescriptor::PuppetItems,
        primary_language: "english",
    },
    ResourceSpec {
        name: "currency",
        descriptor: DatDescriptor::Currency,
        primary_language: "english",
    },
];

#[derive(Debug, Serialize)]
struct ItemsReport {
    schema_version: u32,
    resources: Vec<ItemResource>,
}

#[derive(Debug, Serialize)]
struct ItemResource {
    name: &'static str,
    sources: Vec<ItemSource>,
    entries: Vec<LocalizedItemEntry>,
}

#[derive(Debug, Serialize)]
struct ItemSource {
    language: &'static str,
    dat_id: u32,
    logical_path: String,
    selected_path: String,
}

#[derive(Debug, Serialize)]
struct LocalizedItemEntry {
    index: usize,
    #[serde(flatten)]
    data: ItemData,
    #[serde(skip_serializing_if = "LocalizedItemText::is_empty")]
    text: LocalizedItemText,
}

#[derive(Debug, Default, Serialize)]
struct LocalizedItemText {
    #[serde(skip_serializing_if = "Option::is_none")]
    english: Option<ItemTextData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    japanese: Option<ItemTextData>,
}

impl LocalizedItemText {
    fn is_empty(&self) -> bool {
        self.english.is_none() && self.japanese.is_none()
    }

    fn set(&mut self, language: &str, text: Option<ItemTextData>) -> Result<()> {
        if let Some(text) = &text {
            let is_english = text.article_type_code.is_some()
                && text.singular_name.is_some()
                && text.plural_name.is_some()
                && text.description.is_some();
            let is_japanese = text.article_type_code.is_none()
                && text.singular_name.is_none()
                && text.plural_name.is_none()
                && text.description.is_some();
            match language {
                "english" if !is_english => bail!("English item source has non-English text"),
                "japanese" if !is_japanese => bail!("Japanese item source has non-Japanese text"),
                _ => {}
            }
        }
        match language {
            "english" => self.english = text,
            "japanese" => self.japanese = text,
            _ => bail!("Unsupported item language {language}"),
        }
        Ok(())
    }
}

struct DecodedSource {
    metadata: ItemSource,
    entries: Vec<ItemEntry>,
}

fn decode_source(
    runtime_root: &Path,
    id_map: &HashMap<DatId, DatPath>,
    dat_id: DatId,
    language: &'static str,
    resource_name: &str,
) -> Result<DecodedSource> {
    let logical_path = id_map.get(&dat_id).copied().ok_or_else(|| {
        anyhow!(
            "Client mapping does not contain {resource_name} {language} DAT ID {}",
            dat_id.get_inner()
        )
    })?;
    let selected_path = resolve_xbox_source(runtime_root, logical_path).ok_or_else(|| {
        anyhow!(
            "Client mapping does not contain an accepted source for {resource_name} {language} DAT ID {}",
            dat_id.get_inner()
        )
    })?;
    let table =
        ItemInfoTable::from_path(&runtime_root.join(&selected_path)).with_context(|| {
            format!(
                "Could not decode {resource_name} {language} from {}",
                stable_relative_path(&selected_path)
            )
        })?;

    Ok(DecodedSource {
        metadata: ItemSource {
            language,
            dat_id: dat_id.get_inner(),
            logical_path: stable_relative_path(&logical_path.to_path()),
            selected_path: stable_relative_path(&selected_path),
        },
        entries: table.neutral_entries(),
    })
}

fn build_resource(
    runtime_root: &Path,
    id_map: &HashMap<DatId, DatPath>,
    spec: &ResourceSpec,
) -> Result<ItemResource> {
    let primary_id = spec.descriptor.dat_id()?;
    let primary = decode_source(
        runtime_root,
        id_map,
        primary_id,
        spec.primary_language,
        spec.name,
    )?;
    let japanese = spec
        .descriptor
        .has_jp_dat()
        .then(|| {
            decode_source(
                runtime_root,
                id_map,
                spec.descriptor.jp_dat_id()?,
                "japanese",
                spec.name,
            )
        })
        .transpose()?;

    let entries = merge_entries(
        spec.name,
        spec.primary_language,
        primary.entries,
        japanese.as_ref().map(|source| source.entries.as_slice()),
    )?;

    let mut sources = vec![primary.metadata];
    if let Some(japanese) = japanese {
        sources.push(japanese.metadata);
    }

    Ok(ItemResource {
        name: spec.name,
        sources,
        entries,
    })
}

fn merge_entries(
    resource_name: &str,
    primary_language: &str,
    primary_entries: Vec<ItemEntry>,
    japanese_entries: Option<&[ItemEntry]>,
) -> Result<Vec<LocalizedItemEntry>> {
    if let Some(japanese_entries) = japanese_entries
        && primary_entries.len() != japanese_entries.len()
    {
        bail!(
            "{} language tables have different entry counts: {} and {}",
            resource_name,
            primary_entries.len(),
            japanese_entries.len()
        );
    }

    let mut entries = Vec::with_capacity(primary_entries.len());
    for (index, primary_entry) in primary_entries.into_iter().enumerate() {
        let mut text = LocalizedItemText::default();
        text.set(primary_language, primary_entry.text)?;
        if let Some(japanese_entry) = japanese_entries.and_then(|entries| entries.get(index)) {
            if primary_entry.data != japanese_entry.data {
                bail!(
                    "{} language payloads differ at entry {} (item IDs {} and {})",
                    resource_name,
                    index,
                    primary_entry.data.id,
                    japanese_entry.data.id
                );
            }
            text.set("japanese", japanese_entry.text.clone())?;
        }
        entries.push(LocalizedItemEntry {
            index,
            data: primary_entry.data,
            text,
        });
    }
    Ok(entries)
}

pub fn export_items(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let resources = RESOURCE_SPECS
        .iter()
        .map(|spec| build_resource(&runtime_root, &id_map, spec))
        .collect::<Result<Vec<_>>>()?;
    let report = ItemsReport {
        schema_version: REPORT_SCHEMA_VERSION,
        resources,
    };
    let mut json = serde_json::to_string_pretty(&report)?;
    json.push('\n');

    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use dats::dat_format::DatFormat;

    use super::*;

    fn item_entry(id: u32, name: &str) -> ItemEntry {
        ItemEntry {
            data: ItemData {
                id,
                flags_bits: 0,
                stack_size: 1,
                item_type_code: 1,
                resource_id: 0,
                valid_targets_bits: 0,
                equipment: None,
                weapon: None,
                puppet: None,
                instinct: None,
                furnishing: None,
                usable: None,
                currency: None,
                slip: None,
                monipulator: None,
            },
            text: Some(ItemTextData {
                name: name.to_string(),
                article_type_code: Some(0),
                singular_name: Some(name.to_string()),
                plural_name: Some(format!("{name}s")),
                description: Some("Synthetic item.".to_string()),
            }),
        }
    }

    fn japanese_entry(id: u32, name: &str) -> ItemEntry {
        let mut entry = item_entry(id, name);
        entry.text = Some(ItemTextData {
            name: name.to_string(),
            article_type_code: None,
            singular_name: None,
            plural_name: None,
            description: Some("Synthetic item.".to_string()),
        });
        entry
    }

    fn table_bytes(id: u32, item_type: &str, language: &str, payload: &str) -> Vec<u8> {
        let strings = match language {
            "english" => concat!(
                "    strings:\n",
                "      name: Synthetic item\n",
                "      article_type: A\n",
                "      singular_name: synthetic item\n",
                "      plural_name: synthetic items\n",
                "      description: Synthetic description.\n"
            ),
            "japanese" => concat!(
                "    strings:\n",
                "      name: Synthetic JP item\n",
                "      description: Synthetic JP description.\n"
            ),
            _ => panic!("unsupported test language"),
        };
        let yaml = format!(
            concat!(
                "items:\n",
                "  - id: {id}\n",
                "{strings}",
                "    flags: []\n",
                "    stack_size: 1\n",
                "    item_type: {item_type}\n",
                "    resource_id: 0\n",
                "    valid_targets: []\n",
                "{payload}",
                "    icon_bytes: \"\"\n"
            ),
            id = id,
            strings = strings,
            item_type = item_type,
            payload = payload
        );
        let table: ItemInfoTable = serde_yaml::from_str(&yaml).unwrap();
        table.to_bytes().unwrap()
    }

    fn write_full_test_runtime(root: &Path) {
        let general = concat!(
            "    furnishing:\n",
            "      element: Fire\n",
            "      storage_slots: 0\n"
        );
        let usable = concat!(
            "    usable_item:\n",
            "      activation_time: 0\n",
            "      unknown1: 0\n"
        );
        let equipment = concat!(
            "    equipment:\n",
            "      level: 1\n",
            "      slots: []\n",
            "      races: []\n",
            "      jobs: []\n",
            "      max_charges: 0\n",
            "      casting_time: 0\n",
            "      use_delay: 0\n",
            "      reuse_delay: 0\n",
            "      unknown1: 0\n",
            "      ilevel: 0\n",
            "      unknown2: 0\n",
            "      unknown3: 0\n"
        );
        let weapon = concat!(
            "    equipment:\n",
            "      level: 1\n",
            "      slots: []\n",
            "      races: []\n",
            "      jobs: []\n",
            "      max_charges: 0\n",
            "      casting_time: 0\n",
            "      use_delay: 0\n",
            "      reuse_delay: 0\n",
            "      unknown1: 0\n",
            "      ilevel: 0\n",
            "      unknown2: 0\n",
            "      unknown3: 0\n",
            "    weapon:\n",
            "      damage: 1\n",
            "      delay: 240\n",
            "      dps: 1\n",
            "      skill_type: Sword\n",
            "      jug_size: 0\n"
        );
        let puppet = concat!(
            "    puppet:\n",
            "      slot: Head\n",
            "      element_charge:\n",
            "        fire: 0\n",
            "        ice: 0\n",
            "        wind: 0\n",
            "        earth: 0\n",
            "        lightning: 0\n",
            "        water: 0\n",
            "        light: 0\n",
            "        dark: 0\n",
            "      unknown1: 0\n"
        );
        let currency = "    currency:\n      unknown1: 0\n";
        let specs = [
            (4usize, 1u32, "Item", "japanese", general),
            (73, 1, "Item", "english", general),
            (5, 0x1000, "UsableItem", "japanese", usable),
            (74, 0x1000, "UsableItem", "english", usable),
            (6, 0x4000, "Weapon", "japanese", weapon),
            (75, 0x4000, "Weapon", "english", weapon),
            (7, 0x2800, "Armor", "japanese", equipment),
            (76, 0x2800, "Armor", "english", equipment),
            (8, 0x2000, "PuppetItem", "japanese", puppet),
            (77, 0x2000, "PuppetItem", "english", puppet),
            (91, 0xFFFF, "Currency", "english", currency),
        ];
        let mut vtable = vec![0u8; 92];
        let mut ftable = vec![0u8; 92 * 2];
        fs::create_dir_all(root.join("0001")).unwrap();
        for (dat_id, item_id, item_type, language, payload) in specs {
            vtable[dat_id] = 1;
            ftable[dat_id * 2..dat_id * 2 + 2].copy_from_slice(&(dat_id as u16).to_le_bytes());
            let package = format!("R0001{:02}", (1 + dat_id) % 12);
            let path = root
                .join(package)
                .join("ROM/0")
                .join(format!("{dat_id}.DAT"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, table_bytes(item_id, item_type, language, payload)).unwrap();
        }
        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
    }

    #[test]
    fn language_tables_pair_by_index_even_with_duplicate_ids() {
        let primary = vec![item_entry(0xFFFF, "first"), item_entry(0xFFFF, "second")];
        let japanese = vec![japanese_entry(0xFFFF, "ichi"), japanese_entry(0xFFFF, "ni")];

        let entries = merge_entries("currency", "english", primary, Some(&japanese)).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[0].data.id, 0xFFFF);
        assert_eq!(entries[0].text.english.as_ref().unwrap().name, "first");
        assert_eq!(entries[0].text.japanese.as_ref().unwrap().name, "ichi");
    }

    #[test]
    fn language_payload_mismatch_is_rejected() {
        let primary = vec![item_entry(1, "item")];
        let mut japanese = vec![japanese_entry(1, "item jp")];
        japanese[0].data.stack_size = 12;

        let error = merge_entries("general_items", "english", primary, Some(&japanese))
            .unwrap_err()
            .to_string();

        assert!(error.contains("payloads differ at entry 0"));
    }

    #[test]
    fn unexpected_language_shape_is_rejected() {
        let japanese = vec![item_entry(1, "not japanese")];

        let error = merge_entries("general_items", "japanese", japanese, None)
            .unwrap_err()
            .to_string();

        assert_eq!(error, "Japanese item source has non-Japanese text");
    }

    #[test]
    fn empty_text_is_omitted() {
        let mut entry = item_entry(1, "unused");
        entry.text = None;
        let entries = merge_entries("general_items", "english", vec![entry], None).unwrap();

        let json = serde_json::to_string(&entries).unwrap();

        assert!(!json.contains("text"));
        assert!(!json.contains("icon"));
        assert!(!json.contains("raw_strings"));
    }

    #[test]
    fn missing_selected_source_is_an_error() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-items-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("0001")).unwrap();
        let mut vtable = vec![0u8; 74];
        let mut ftable = vec![0u8; 74 * 2];
        vtable[73] = 1;
        ftable[73 * 2..73 * 2 + 2].copy_from_slice(&73u16.to_le_bytes());
        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();

        let error = export_items(root.clone(), root.join("items.json"))
            .unwrap_err()
            .to_string();

        assert!(error.contains("general_items english DAT ID 73"));
        assert!(!error.contains(&root.to_string_lossy().to_string()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn full_export_is_deterministic_relative_and_complete() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-items-full-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_full_test_runtime(&root);
        let first_path = root.join("first.json");
        let second_path = root.join("second.json");

        export_items(root.clone(), first_path.clone()).unwrap();
        export_items(root.clone(), second_path.clone()).unwrap();
        let first = fs::read_to_string(first_path).unwrap();
        let second = fs::read_to_string(second_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();

        assert_eq!(first, second);
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["resources"].as_array().unwrap().len(), 6);
        assert_eq!(report["resources"][0]["name"], "general_items");
        assert_eq!(report["resources"][0]["sources"][0]["dat_id"], 73);
        assert_eq!(
            report["resources"][0]["sources"][0]["selected_path"],
            "R000102/ROM/0/73.DAT"
        );
        assert_eq!(report["resources"][0]["sources"][1]["dat_id"], 4);
        assert_eq!(report["resources"][5]["name"], "currency");
        assert_eq!(report["resources"][5]["sources"][0]["language"], "english");
        assert!(
            report["resources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|resource| resource["entries"].as_array().unwrap().len() == 1)
        );
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert!(!first.contains("icon_bytes"));
        assert!(!first.contains("raw_strings"));
        fs::remove_dir_all(root).unwrap();
    }
}
