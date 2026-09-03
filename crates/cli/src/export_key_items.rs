use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use dats::{
    base::{DatId, DatPath},
    context::DatContext,
    dat_format::DatFormat,
    formats::{
        dmsg_list::{DmsgContent, DmsgEntryList},
        dmsg_table::DmsgTable,
    },
    id_mapping::DatDescriptor,
};
use serde::Serialize;

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct KeyItemsReport {
    schema_version: u32,
    resources: Vec<KeyItemResource>,
}

#[derive(Debug, Serialize)]
struct KeyItemResource {
    name: &'static str,
    sources: Vec<KeyItemSource>,
    entries: Vec<KeyItemEntry>,
}

#[derive(Debug, Serialize)]
struct KeyItemSource {
    language: &'static str,
    dat_id: u32,
    logical_path: String,
    selected_path: String,
}

#[derive(Debug, Serialize)]
struct KeyItemEntry {
    index: u32,
    // A null ID preserves a DAT entry with missing numeric content for later
    // validation instead of silently dropping it from the catalog.
    id: Option<u32>,
    text: LocalizedKeyItemText,
}

#[derive(Debug, Default, Serialize)]
struct LocalizedKeyItemText {
    #[serde(skip_serializing_if = "Option::is_none")]
    english: Option<KeyItemText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    japanese: Option<KeyItemText>,
}

#[derive(Debug, Default, Serialize)]
struct KeyItemText {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plural_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

struct DecodedSource {
    metadata: KeyItemSource,
    lists: Vec<(u32, DmsgEntryList)>,
}

fn decode_source(
    runtime_root: &Path,
    id_map: &std::collections::HashMap<DatId, DatPath>,
    dat_id: DatId,
    language: &'static str,
) -> Result<DecodedSource> {
    let logical_path = id_map.get(&dat_id).copied().ok_or_else(|| {
        anyhow!(
            "Client mapping does not contain key items {language} DAT ID {}",
            dat_id.get_inner()
        )
    })?;
    let selected_path = resolve_xbox_source(runtime_root, logical_path).ok_or_else(|| {
        anyhow!(
            "Client mapping does not contain an accepted source for key items {language} DAT ID {}",
            dat_id.get_inner()
        )
    })?;
    let table = DmsgTable::from_path(&runtime_root.join(&selected_path)).with_context(|| {
        format!(
            "Could not decode key items {language} from {}",
            stable_relative_path(&selected_path)
        )
    })?;

    Ok(DecodedSource {
        metadata: KeyItemSource {
            language,
            dat_id: dat_id.get_inner(),
            logical_path: stable_relative_path(&logical_path.to_path()),
            selected_path: stable_relative_path(&selected_path),
        },
        lists: table.lists.into_iter().collect(),
    })
}

fn string_at(list: Option<&DmsgEntryList>, index: usize) -> Option<String> {
    list?.content.get(index).and_then(|content| match content {
        DmsgContent::String { string } if !string.is_empty() => Some(string.clone()),
        _ => None,
    })
}

fn id_at(list: Option<&DmsgEntryList>) -> Option<u32> {
    list?.content.first().and_then(|content| match content {
        DmsgContent::Number { number } => Some(*number),
        DmsgContent::String { .. } => None,
    })
}

fn text_at(list: Option<&DmsgEntryList>, language: &str) -> Option<KeyItemText> {
    let (name_index, plural_name_index, description_index) = match language {
        "english" => (4, Some(5), 6),
        // Japanese key-item lists contain a name and description after the ID;
        // unlike English lists, they do not contain article/plural fields.
        "japanese" => (1, None, 2),
        _ => return None,
    };
    let text = KeyItemText {
        name: string_at(list, name_index),
        plural_name: plural_name_index.and_then(|index| string_at(list, index)),
        description: string_at(list, description_index),
    };
    (text.name.is_some() || text.plural_name.is_some() || text.description.is_some())
        .then_some(text)
}

fn merge_entries(english: DecodedSource, japanese: DecodedSource) -> Result<KeyItemResource> {
    if english.lists.len() != japanese.lists.len() {
        bail!(
            "Key item language tables have different entry counts: {} and {}",
            english.lists.len(),
            japanese.lists.len()
        );
    }

    let entries = english
        .lists
        .iter()
        .zip(japanese.lists.iter())
        .map(
            |((english_index, english_list), (japanese_index, japanese_list))| {
                if english_index != japanese_index {
                    return Err(anyhow!(
                        "Key item language tables differ at entry indexes {} and {}",
                        english_index,
                        japanese_index
                    ));
                }
                let english_id = id_at(Some(english_list));
                let japanese_id = id_at(Some(japanese_list));
                if let (Some(english_id), Some(japanese_id)) = (english_id, japanese_id)
                    && english_id != japanese_id
                {
                    bail!(
                        "Key item language payloads differ at entry {} (IDs {} and {})",
                        english_index,
                        english_id,
                        japanese_id
                    );
                }
                Ok(KeyItemEntry {
                    index: *english_index,
                    id: english_id.or(japanese_id),
                    text: LocalizedKeyItemText {
                        english: text_at(Some(english_list), "english"),
                        japanese: text_at(Some(japanese_list), "japanese"),
                    },
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;

    Ok(KeyItemResource {
        name: "key_items",
        sources: vec![english.metadata, japanese.metadata],
        entries,
    })
}

pub fn export_key_items(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let descriptor = DatDescriptor::KeyItems;
    let english = decode_source(&runtime_root, &id_map, descriptor.dat_id()?, "english")?;
    let japanese = decode_source(&runtime_root, &id_map, descriptor.jp_dat_id()?, "japanese")?;
    let report = KeyItemsReport {
        schema_version: REPORT_SCHEMA_VERSION,
        resources: vec![merge_entries(english, japanese)?],
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
    use std::{collections::BTreeMap, fs};

    use dats::formats::dmsg_list::DmsgEntryList;

    use super::*;

    fn key_item_list(
        id: Option<u32>,
        name: Option<&str>,
        description: Option<&str>,
    ) -> DmsgEntryList {
        let content = vec![
            id.map_or(
                DmsgContent::String {
                    string: String::new(),
                },
                |number| DmsgContent::Number { number },
            ),
            DmsgContent::Number { number: 2 },
            DmsgContent::String {
                string: String::new(),
            },
            DmsgContent::String {
                string: String::new(),
            },
            DmsgContent::String {
                string: name.unwrap_or_default().to_string(),
            },
            DmsgContent::String {
                string: name.unwrap_or_default().to_string(),
            },
            DmsgContent::String {
                string: description.unwrap_or_default().to_string(),
            },
        ];
        DmsgEntryList { content }
    }

    fn japanese_key_item_list(
        id: Option<u32>,
        name: &str,
        description: Option<&str>,
    ) -> DmsgEntryList {
        DmsgEntryList {
            content: vec![
                id.map_or(
                    DmsgContent::String {
                        string: String::new(),
                    },
                    |number| DmsgContent::Number { number },
                ),
                DmsgContent::String {
                    string: name.to_string(),
                },
                DmsgContent::String {
                    string: description.unwrap_or_default().to_string(),
                },
            ],
        }
    }

    fn table_bytes(lists: BTreeMap<u32, DmsgEntryList>) -> Vec<u8> {
        DmsgTable {
            bytes_per_list: 0,
            flip_bytes: false,
            lists,
        }
        .to_bytes()
        .unwrap()
    }

    fn write_runtime(
        root: &Path,
        english: BTreeMap<u32, DmsgEntryList>,
        japanese: BTreeMap<u32, DmsgEntryList>,
    ) {
        let mut vtable = vec![0u8; 55_696];
        let mut ftable = vec![0u8; 55_696 * 2];
        let mapped = [(55_695usize, 1u16), (55_575usize, 2u16)];
        for (id, combined_path) in mapped {
            vtable[id] = 1;
            ftable[id * 2..id * 2 + 2].copy_from_slice(&combined_path.to_le_bytes());
        }
        fs::create_dir_all(root.join("0001")).unwrap();
        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
        fs::create_dir_all(root.join("R000102/ROM/0")).unwrap();
        fs::create_dir_all(root.join("R000103/ROM/0")).unwrap();
        fs::write(root.join("R000102/ROM/0/1.DAT"), table_bytes(english)).unwrap();
        fs::write(root.join("R000103/ROM/0/2.DAT"), table_bytes(japanese)).unwrap();
    }

    #[test]
    fn export_is_deterministic_and_preserves_missing_ids() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-key-items-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_runtime(
            &root,
            BTreeMap::from([
                (
                    0,
                    key_item_list(
                        Some(1060),
                        Some("Conquest promotion voucher"),
                        Some("Synthetic description."),
                    ),
                ),
                (1, key_item_list(None, Some("Missing ID"), None)),
            ]),
            BTreeMap::from([
                (
                    0,
                    japanese_key_item_list(Some(1060), "Synthetic JP key item", None),
                ),
                (1, japanese_key_item_list(None, "Missing ID", None)),
            ]),
        );

        let output_path = root.join("out/key-items.json");
        export_key_items(root.clone(), output_path.clone()).unwrap();
        let first = fs::read_to_string(&output_path).unwrap();
        export_key_items(root.clone(), output_path.clone()).unwrap();
        let second = fs::read_to_string(&output_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["resources"][0]["name"], "key_items");
        assert_eq!(report["resources"][0]["entries"][0]["id"], 1060);
        assert_eq!(
            report["resources"][0]["entries"][0]["text"]["english"]["name"],
            "Conquest promotion voucher"
        );
        assert_eq!(
            report["resources"][0]["entries"][0]["text"]["japanese"]["name"],
            "Synthetic JP key item"
        );
        assert!(report["resources"][0]["entries"][1]["id"].is_null());
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mismatched_language_ids_are_an_error() {
        let english = DecodedSource {
            metadata: KeyItemSource {
                language: "english",
                dat_id: 55_695,
                logical_path: "ROM/0/1.DAT".to_string(),
                selected_path: "R000102/ROM/0/1.DAT".to_string(),
            },
            lists: vec![(0, key_item_list(Some(1060), Some("English"), None))],
        };
        let japanese = DecodedSource {
            metadata: KeyItemSource {
                language: "japanese",
                dat_id: 55_575,
                logical_path: "ROM/0/2.DAT".to_string(),
                selected_path: "R000103/ROM/0/2.DAT".to_string(),
            },
            lists: vec![(0, key_item_list(Some(1061), Some("Japanese"), None))],
        };

        let error = merge_entries(english, japanese).unwrap_err();
        assert!(error.to_string().contains("entry 0"));
        assert!(error.to_string().contains("1060"));
        assert!(error.to_string().contains("1061"));
    }

    #[test]
    fn missing_selected_source_is_an_error_without_absolute_root() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-key-items-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_runtime(
            &root,
            BTreeMap::from([(0, key_item_list(Some(1060), Some("English"), None))]),
            BTreeMap::from([(0, japanese_key_item_list(Some(1060), "Japanese", None))]),
        );
        fs::remove_file(root.join("R000102/ROM/0/1.DAT")).unwrap();

        let error = export_key_items(root.clone(), root.join("key-items.json")).unwrap_err();
        assert!(error.to_string().contains("key items english DAT ID 55695"));
        assert!(
            !error
                .to_string()
                .contains(&root.to_string_lossy().to_string())
        );

        fs::remove_dir_all(root).unwrap();
    }
}
