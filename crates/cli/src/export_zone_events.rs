use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use dats::{
    base::ZoneId, context::DatContext, dat_format::DatFormat, formats::events::Events,
    id_mapping::DatDescriptor,
};
use serde::{Deserialize, Serialize};

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;
const ZONE_COUNT: u16 = 256;

#[derive(Debug, Serialize)]
struct ZoneEventsReport {
    schema_version: u32,
    zones: Vec<ZoneEventsResource>,
}

#[derive(Debug, Serialize)]
struct ZoneEventsResource {
    zone_id: ZoneId,
    dat_id: u32,
    logical_path: String,
    selected_path: String,
    event_blocks: Vec<ZoneEventBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZoneEventBlock {
    entity_id: u32,
    events: Vec<ZoneEvent>,
    data: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZoneEvent {
    id: u16,
    byte_code: String,
}

#[derive(Debug, Deserialize)]
struct DecodedZoneEvents {
    blocks: Vec<ZoneEventBlock>,
}

/// Export event blocks for the 256 primary zones from an Xbox package runtime.
///
/// Event DATs are language-neutral. The report preserves the event byte code as
/// stable hexadecimal so an offline reconciler can compare event definitions,
/// not just their IDs and entity associations.
pub fn export_zone_events(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let zones = (0..ZONE_COUNT)
        .map(|zone_id| {
            let descriptor = DatDescriptor::Events(zone_id);
            let dat_id = descriptor.dat_id()?;
            let dat_path = id_map.get(&dat_id).copied().ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain event DAT ID {} for zone {}",
                    dat_id.get_inner(),
                    zone_id
                )
            })?;
            let selected_path = resolve_xbox_source(&runtime_root, dat_path).ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain an accepted source for event DAT ID {} for zone {}",
                    dat_id.get_inner(),
                    zone_id
                )
            })?;
            let events = Events::from_path(&runtime_root.join(&selected_path)).with_context(|| {
                format!(
                    "Could not decode zone {} events from {}",
                    zone_id,
                    stable_relative_path(&selected_path)
                )
            })?;
            let event_blocks = serde_json::from_value::<DecodedZoneEvents>(serde_json::to_value(
                events,
            )?)?
            .blocks;

            Ok(ZoneEventsResource {
                zone_id,
                dat_id: dat_id.get_inner(),
                logical_path: stable_relative_path(&dat_path.to_path()),
                selected_path: stable_relative_path(&selected_path),
                event_blocks,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let report = ZoneEventsReport {
        schema_version: REPORT_SCHEMA_VERSION,
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

    fn event_bytes(zone_id: u16, include_block: bool) -> Vec<u8> {
        if !include_block {
            return 0u32.to_le_bytes().to_vec();
        }

        let byte_code = [0x10, 0x20, 0x30, 0x40, 0x50];
        let mut block = Vec::new();
        block.extend_from_slice(&(0x0100_0000 | u32::from(zone_id)).to_le_bytes());
        block.extend_from_slice(&2u32.to_le_bytes());
        block.extend_from_slice(&0u16.to_le_bytes());
        block.extend_from_slice(&3u16.to_le_bytes());
        block.extend_from_slice(&0x0123u16.to_le_bytes());
        block.extend_from_slice(&0x0456u16.to_le_bytes());
        block.extend_from_slice(&1u32.to_le_bytes());
        block.extend_from_slice(&0xAABB_CCDDu32.to_le_bytes());
        block.extend_from_slice(&(byte_code.len() as u32).to_le_bytes());
        block.extend_from_slice(&byte_code);
        block.extend(std::iter::repeat_n(0xFF, (4 - (byte_code.len() & 3)) & 3));

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(block.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&block);
        bytes
    }

    fn write_runtime(root: &PathBuf) {
        let max_dat_id = 5820usize + ZONE_COUNT as usize;
        let mut vtable = vec![0u8; max_dat_id];
        let mut ftable = vec![0u8; max_dat_id * 2];
        fs::create_dir_all(root.join("0001")).unwrap();

        for zone_id in 0..ZONE_COUNT {
            let dat_id = 5820usize + zone_id as usize;
            let folder_id = 240u16 + zone_id / 128;
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
                event_bytes(zone_id, !EMPTY_ZONE_IDS.contains(&zone_id)),
            )
            .unwrap();
        }

        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
    }

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xi-tinkerer-zone-events-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn export_is_deterministic_relative_and_keeps_empty_zones() {
        let root = temp_root("deterministic");
        write_runtime(&root);

        let output_path = root.join("out/zone-events.json");
        export_zone_events(root.clone(), output_path.clone()).unwrap();
        let first = fs::read_to_string(&output_path).unwrap();
        export_zone_events(root.clone(), output_path.clone()).unwrap();
        let second = fs::read_to_string(&output_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();
        let zones = report["zones"].as_array().unwrap();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(zones.len(), ZONE_COUNT as usize);
        assert_eq!(zones[0]["zone_id"], 0);
        assert_eq!(zones[0]["dat_id"], 5820);
        let empty_zone_ids = zones
            .iter()
            .filter(|zone| zone["event_blocks"].as_array().unwrap().is_empty())
            .map(|zone| zone["zone_id"].as_u64().unwrap() as u16)
            .collect::<Vec<_>>();
        assert_eq!(empty_zone_ids, EMPTY_ZONE_IDS);
        assert_eq!(zones[1]["event_blocks"][0]["entity_id"], 0x0100_0001u32);
        assert_eq!(zones[1]["event_blocks"][0]["events"][0]["id"], 0x0123);
        assert_eq!(
            zones[1]["event_blocks"][0]["events"][0]["byte_code"],
            "0x102030"
        );
        assert_eq!(
            zones[1]["event_blocks"][0]["events"][1]["byte_code"],
            "0x4050"
        );
        assert_eq!(zones[1]["event_blocks"][0]["data"][0], 0xAABB_CCDD_u32);
        assert!(
            zones[0]["selected_path"]
                .as_str()
                .unwrap()
                .starts_with("R0001")
        );
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_selected_source_is_an_error() {
        let root = temp_root("missing");
        write_runtime(&root);
        fs::remove_file(root.join("R000102/ROM/240/1.DAT")).unwrap();

        let error = export_zone_events(root.clone(), root.join("zone-events.json")).unwrap_err();

        assert!(error.to_string().contains("zone 1"));
        assert!(
            !error
                .to_string()
                .contains(&root.to_string_lossy().to_string())
        );

        fs::remove_dir_all(root).unwrap();
    }
}
