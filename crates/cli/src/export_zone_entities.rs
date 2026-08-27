use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use dats::{
    base::ZoneId, context::DatContext, dat_format::DatFormat, formats::entity_names::EntityNames,
    id_mapping::DatDescriptor,
};
use serde::Serialize;

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;
const ZONE_COUNT: u16 = 256;

#[derive(Debug, Serialize)]
struct ZoneEntitiesReport {
    schema_version: u32,
    language: &'static str,
    zones: Vec<ZoneEntitiesResource>,
}

#[derive(Debug, Serialize)]
struct ZoneEntitiesResource {
    zone_id: ZoneId,
    dat_id: u32,
    logical_path: String,
    selected_path: String,
    entries: Vec<ZoneEntityEntry>,
}

#[derive(Debug, Serialize)]
struct ZoneEntityEntry {
    id: u32,
    name: String,
}

pub fn export_zone_entities(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let zones = (0..ZONE_COUNT)
        .map(|zone_id| {
            let descriptor = DatDescriptor::EntityNames(zone_id);
            let dat_id = descriptor.dat_id()?;
            let dat_path = id_map.get(&dat_id).copied().ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain entity DAT ID {} for zone {}",
                    dat_id.get_inner(),
                    zone_id
                )
            })?;
            let selected_path = resolve_xbox_source(&runtime_root, dat_path).ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain an accepted source for entity DAT ID {} for zone {}",
                    dat_id.get_inner(),
                    zone_id
                )
            })?;
            let entities = EntityNames::from_path(&runtime_root.join(&selected_path))
                .with_context(|| {
                    format!(
                        "Could not decode zone {} entities from {}",
                        zone_id,
                        stable_relative_path(&selected_path)
                    )
                })?;
            let entries = entities
                .names
                .iter()
                .map(|entity| ZoneEntityEntry {
                    id: entity.id(),
                    name: entity.name().to_string(),
                })
                .collect();

            Ok(ZoneEntitiesResource {
                zone_id,
                dat_id: dat_id.get_inner(),
                logical_path: stable_relative_path(&dat_path.to_path()),
                selected_path: stable_relative_path(&selected_path),
                entries,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let report = ZoneEntitiesReport {
        schema_version: REPORT_SCHEMA_VERSION,
        language: "english",
        zones,
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
    use super::*;

    const EMPTY_ZONE_IDS: [u16; 19] = [
        0, 15, 45, 49, 132, 133, 189, 199, 214, 215, 216, 217, 218, 219, 222, 229, 253, 254, 255,
    ];

    fn entity_bytes(zone_id: u16, name: Option<&str>) -> Vec<u8> {
        let mut bytes = b"none".to_vec();
        bytes.resize(28, 0);
        if let Some(name) = name {
            bytes.extend_from_slice(name.as_bytes());
            bytes.resize(52, 0);
            bytes.extend_from_slice(&(0x01000000 | (zone_id as u32) << 12 | 1).to_le_bytes());
        }
        bytes
    }

    fn write_runtime(root: &PathBuf) {
        let mut vtable = vec![0u8; 6_976];
        let mut ftable = vec![0u8; 6_976 * 2];
        fs::create_dir_all(root.join("0001")).unwrap();

        for zone_id in 0..ZONE_COUNT {
            let dat_id = 6_720usize + zone_id as usize;
            let folder_id = 210u16 + zone_id / 128;
            let file_id = zone_id % 128;
            let combined_path = (folder_id << 7) | file_id;
            vtable[dat_id] = 1;
            ftable[dat_id * 2..dat_id * 2 + 2].copy_from_slice(&combined_path.to_le_bytes());

            let package_index = (1usize + folder_id as usize * 128 + file_id as usize) % 12;
            let package = format!("R{:06}", 100 + package_index);
            let relative_path = format!("{package}/ROM/{folder_id}/{file_id}.DAT");
            let path = root.join(relative_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(
                path,
                entity_bytes(
                    zone_id,
                    (!EMPTY_ZONE_IDS.contains(&zone_id)).then_some("Synthetic Entity"),
                ),
            )
            .unwrap();
        }

        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
    }

    #[test]
    fn export_is_deterministic_relative_and_keeps_empty_zones() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-zone-entities-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_runtime(&root);

        let output_path = root.join("out/zone-entities.json");
        export_zone_entities(root.clone(), output_path.clone()).unwrap();
        let first = fs::read_to_string(&output_path).unwrap();
        export_zone_entities(root.clone(), output_path.clone()).unwrap();
        let second = fs::read_to_string(&output_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();
        let zones = report["zones"].as_array().unwrap();
        let empty_zone_ids = zones
            .iter()
            .filter(|zone| zone["entries"].as_array().unwrap().is_empty())
            .map(|zone| zone["zone_id"].as_u64().unwrap() as u16)
            .collect::<Vec<_>>();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["language"], "english");
        assert_eq!(zones.len(), ZONE_COUNT as usize);
        assert_eq!(empty_zone_ids, EMPTY_ZONE_IDS);
        assert_eq!(zones[1]["zone_id"], 1);
        assert_eq!(zones[1]["dat_id"], 6_721);
        assert_eq!(zones[1]["entries"][0]["id"], 0x01001001u32);
        assert_eq!(zones[1]["entries"][0]["name"], "Synthetic Entity");
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_selected_source_is_an_error() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-zone-entities-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_runtime(&root);
        fs::remove_file(root.join("R000102/ROM/210/1.DAT")).unwrap();

        let error =
            export_zone_entities(root.clone(), root.join("zone-entities.json")).unwrap_err();

        assert!(error.to_string().contains("zone 1"));
        assert!(
            !error
                .to_string()
                .contains(&root.to_string_lossy().to_string())
        );

        fs::remove_dir_all(root).unwrap();
    }
}
