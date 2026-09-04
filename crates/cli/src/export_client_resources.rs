use std::{
    collections::HashMap,
    fs,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use dats::{
    base::{Dat, DatId, DatPath},
    context::DatContext,
    dat_format::DatFormat,
    id_mapping::{DatDescriptor, DatIdMapping, DatUsage},
};
use serde::Serialize;
use serde_json::Value;

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct ClientResourcesReport {
    schema_version: u32,
    resources: Vec<ClientResource>,
}

#[derive(Debug, Serialize)]
struct ClientResource {
    name: String,
    format: &'static str,
    sources: Vec<ClientResourceSource>,
}

#[derive(Debug, Serialize)]
struct ClientResourceSource {
    language: &'static str,
    dat_id: u32,
    status: SourceStatus,
    logical_path: Option<String>,
    selected_path: Option<String>,
    data: Option<Value>,
    error: Option<String>,
    normalized_catalog: Option<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourceStatus {
    Selected,
    Missing,
    Absent,
    DecodeFailed,
}

struct JsonDatUsage {
    path: PathBuf,
}

impl DatUsage<Value> for JsonDatUsage {
    fn use_dat<T: DatFormat + Serialize + for<'a> serde::Deserialize<'a>>(
        self,
        _dat: Dat<T>,
    ) -> Result<Value> {
        let value = panic::catch_unwind(AssertUnwindSafe(|| T::from_path(&self.path)))
            .map_err(|_| anyhow::anyhow!("DAT decoder panicked"))??;
        serde_json::to_value(value).context("Could not serialize decoded DAT resource")
    }
}

fn snake_case(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for (index, character) in name.char_indices() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

fn decode_source(
    runtime_root: &Path,
    id_map: &HashMap<DatId, DatPath>,
    descriptor: DatDescriptor,
    language: &'static str,
    japanese: bool,
    normalized_catalog: Option<&'static str>,
) -> Result<ClientResourceSource> {
    let dat_id = if japanese {
        descriptor.jp_dat_id()?
    } else {
        descriptor.dat_id()?
    };
    let Some(logical_path) = id_map.get(&dat_id).copied() else {
        return Ok(ClientResourceSource {
            language,
            dat_id: dat_id.get_inner(),
            status: SourceStatus::Absent,
            logical_path: None,
            selected_path: None,
            data: None,
            error: None,
            normalized_catalog,
        });
    };
    let logical_path_string = stable_relative_path(&logical_path.to_path());
    let Some(selected_path) = resolve_xbox_source(runtime_root, logical_path) else {
        return Ok(ClientResourceSource {
            language,
            dat_id: dat_id.get_inner(),
            status: SourceStatus::Missing,
            logical_path: Some(logical_path_string),
            selected_path: None,
            data: None,
            error: None,
            normalized_catalog,
        });
    };
    let selected_path_string = stable_relative_path(&selected_path);
    if normalized_catalog.is_some() {
        return Ok(ClientResourceSource {
            language,
            dat_id: dat_id.get_inner(),
            status: SourceStatus::Selected,
            logical_path: Some(logical_path_string),
            selected_path: Some(selected_path_string),
            data: None,
            error: None,
            normalized_catalog,
        });
    }
    let dat_user = JsonDatUsage {
        path: runtime_root.join(&selected_path),
    };
    let decoded = if japanese {
        descriptor.use_jp_dat_with(dat_user)
    } else {
        descriptor.use_dat_with(dat_user)
    };

    let (status, data, error) = match decoded {
        Ok(data) => (SourceStatus::Selected, Some(data), None),
        Err(error) => (SourceStatus::DecodeFailed, None, Some(error.to_string())),
    };

    Ok(ClientResourceSource {
        language,
        dat_id: dat_id.get_inner(),
        status,
        logical_path: Some(logical_path_string),
        selected_path: Some(selected_path_string),
        data,
        error,
        normalized_catalog,
    })
}

fn normalized_catalog(resource_name: &str) -> Option<&'static str> {
    matches!(
        resource_name,
        "GeneralItems" | "UsableItems" | "Weapons" | "Armor" | "PuppetItems" | "Currency"
    )
    .then_some("items.json")
}

pub fn export_client_resources(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let resources = DatIdMapping::simple_resource_mappings()
        .into_iter()
        .map(|mapping| {
            let normalized_catalog = normalized_catalog(mapping.name);
            let mut sources = vec![decode_source(
                &runtime_root,
                &id_map,
                mapping.descriptor,
                "english",
                false,
                normalized_catalog,
            )?];
            if mapping.descriptor.has_jp_dat() {
                sources.push(decode_source(
                    &runtime_root,
                    &id_map,
                    mapping.descriptor,
                    "japanese",
                    true,
                    normalized_catalog,
                )?);
            }
            Ok(ClientResource {
                name: snake_case(mapping.name),
                format: mapping.format.name(),
                sources,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let report = ClientResourcesReport {
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
    use std::collections::BTreeMap;

    use dats::formats::{
        dmsg_list::{DmsgContent, DmsgEntryList},
        dmsg_table::DmsgTable,
    };

    use super::*;

    fn table_bytes(value: &str) -> Vec<u8> {
        DmsgTable {
            bytes_per_list: 0,
            flip_bytes: false,
            lists: BTreeMap::from([(
                0,
                DmsgEntryList {
                    content: vec![DmsgContent::String {
                        string: value.to_string(),
                    }],
                },
            )]),
        }
        .to_bytes()
        .unwrap()
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xi-tinkerer-client-resources-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_mapping(root: &Path, mappings: &[(usize, u16)]) {
        let table_len = mappings.iter().map(|(dat_id, _)| *dat_id).max().unwrap() + 1;
        let mut vtable = vec![0u8; table_len];
        let mut ftable = vec![0u8; table_len * 2];
        for (dat_id, combined_path) in mappings {
            vtable[*dat_id] = 1;
            ftable[*dat_id * 2..*dat_id * 2 + 2].copy_from_slice(&combined_path.to_le_bytes());
        }
        fs::create_dir_all(root.join("0001/ROM/0")).unwrap();
        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
    }

    #[test]
    fn snake_case_converts_descriptor_names() {
        assert_eq!(snake_case("AbilityNames"), "ability_names");
        assert_eq!(snake_case("MissionsWotg"), "missions_wotg");
    }

    #[test]
    fn export_records_selected_and_absent_resources_deterministically() {
        let root = test_root();
        write_mapping(&root, &[(55_465, 11), (55_701, 13)]);
        fs::write(root.join("0001/ROM/0/11.DAT"), table_bytes("area")).unwrap();

        let output_path = root.join("out/client-resources.json");
        export_client_resources(root.clone(), output_path.clone()).unwrap();
        let first = fs::read_to_string(&output_path).unwrap();
        export_client_resources(root.clone(), output_path.clone()).unwrap();
        let second = fs::read_to_string(&output_path).unwrap();
        let report: Value = serde_json::from_str(&first).unwrap();
        let resources = report["resources"].as_array().unwrap();
        let area_names = resources
            .iter()
            .find(|resource| resource["name"] == "area_names")
            .unwrap();
        let ability_names = resources
            .iter()
            .find(|resource| resource["name"] == "ability_names")
            .unwrap();
        let spell_names = resources
            .iter()
            .find(|resource| resource["name"] == "spell_names")
            .unwrap();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(
            resources.len(),
            DatIdMapping::simple_resource_mappings().len()
        );
        assert_eq!(area_names["sources"][0]["status"], "selected");
        assert_eq!(
            area_names["sources"][0]["selected_path"],
            "0001/ROM/0/11.DAT"
        );
        assert_eq!(
            area_names["sources"][0]["data"]["lists"]["0"][0]["string"],
            "area"
        );
        assert_eq!(ability_names["sources"][0]["status"], "missing");
        assert_eq!(spell_names["sources"][0]["status"], "absent");
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn export_preserves_decode_failures_and_normalized_item_references() {
        let root = test_root();
        write_mapping(&root, &[(55_465, 11), (73, 12)]);
        fs::write(root.join("0001/ROM/0/11.DAT"), b"invalid").unwrap();
        fs::create_dir_all(root.join("R000101/ROM/0")).unwrap();
        fs::write(root.join("R000101/ROM/0/12.DAT"), b"normalized elsewhere").unwrap();

        let output_path = root.join("out/client-resources.json");
        export_client_resources(root.clone(), output_path.clone()).unwrap();
        let report: Value =
            serde_json::from_str(&fs::read_to_string(output_path).unwrap()).unwrap();
        let resources = report["resources"].as_array().unwrap();
        let area_names = resources
            .iter()
            .find(|resource| resource["name"] == "area_names")
            .unwrap();
        let general_items = resources
            .iter()
            .find(|resource| resource["name"] == "general_items")
            .unwrap();

        assert_eq!(area_names["sources"][0]["status"], "decode_failed");
        assert!(area_names["sources"][0]["error"].is_string());
        assert_eq!(general_items["sources"][0]["status"], "selected");
        assert_eq!(
            general_items["sources"][0]["normalized_catalog"],
            "items.json"
        );
        assert!(general_items["sources"][0]["data"].is_null());

        fs::remove_dir_all(root).unwrap();
    }
}
