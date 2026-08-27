use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use dats::{
    context::DatContext,
    dat_format::DatFormat,
    formats::{dmsg_list::DmsgContent, dmsg_table::DmsgTable},
    id_mapping::DatDescriptor,
};
use serde::Serialize;

use crate::audit_dats::{resolve_xbox_source, stable_relative_path};

const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct ClientGlobalsReport {
    schema_version: u32,
    language: &'static str,
    resources: Vec<ClientGlobalResource>,
}

#[derive(Debug, Serialize)]
struct ClientGlobalResource {
    name: &'static str,
    dat_id: u32,
    logical_path: String,
    selected_path: String,
    entries: Vec<ClientGlobalEntry>,
}

#[derive(Debug, Serialize)]
struct ClientGlobalEntry {
    id: u32,
    values: Vec<DmsgContent>,
}

struct ResourceSpec {
    name: &'static str,
    descriptor: DatDescriptor,
}

const RESOURCE_SPECS: [ResourceSpec; 3] = [
    ResourceSpec {
        name: "area_names",
        descriptor: DatDescriptor::AreaNames,
    },
    ResourceSpec {
        name: "titles",
        descriptor: DatDescriptor::Titles,
    },
    ResourceSpec {
        name: "status_names",
        descriptor: DatDescriptor::StatusNames,
    },
];

pub fn export_client_globals(runtime_root: PathBuf, output_path: PathBuf) -> Result<()> {
    let id_map = DatContext::build_rom_id_map(&runtime_root.join("0001"))
        .context("Could not build the Xbox DAT mapping from package 0001")?;
    let resources = RESOURCE_SPECS
        .iter()
        .map(|spec| {
            let dat_id = spec.descriptor.dat_id()?;
            let dat_path = id_map.get(&dat_id).copied().ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain {} DAT ID {}",
                    spec.name,
                    dat_id.get_inner()
                )
            })?;
            let selected_path = resolve_xbox_source(&runtime_root, dat_path).ok_or_else(|| {
                anyhow!(
                    "Client mapping does not contain an accepted source for {} DAT ID {}",
                    spec.name,
                    dat_id.get_inner()
                )
            })?;
            let table =
                DmsgTable::from_path(&runtime_root.join(&selected_path)).with_context(|| {
                    format!(
                        "Could not decode {} from {}",
                        spec.name,
                        stable_relative_path(&selected_path)
                    )
                })?;
            let entries = table
                .lists
                .into_iter()
                .map(|(id, list)| ClientGlobalEntry {
                    id,
                    values: list.content,
                })
                .collect();

            Ok(ClientGlobalResource {
                name: spec.name,
                dat_id: dat_id.get_inner(),
                logical_path: stable_relative_path(&dat_path.to_path()),
                selected_path: stable_relative_path(&selected_path),
                entries,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let report = ClientGlobalsReport {
        schema_version: REPORT_SCHEMA_VERSION,
        language: "english",
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

    use dats::formats::dmsg_list::DmsgEntryList;

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

    fn write_tables(root: &PathBuf) {
        let mapped = [
            (55_465usize, 11u16),
            (55_704usize, 0u16),
            (55_725usize, 1u16),
        ];
        let mut vtable = vec![0u8; 55_726];
        let mut ftable = vec![0u8; 55_726 * 2];
        for (id, combined_path) in mapped {
            vtable[id] = 1;
            ftable[id * 2..id * 2 + 2].copy_from_slice(&combined_path.to_le_bytes());
        }
        fs::create_dir_all(root.join("0001")).unwrap();
        fs::write(root.join("0001/VTABLE.DAT"), vtable).unwrap();
        fs::write(root.join("0001/FTABLE.DAT"), ftable).unwrap();
    }

    fn write_dat(root: &PathBuf, relative_path: &str, value: &str) {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, table_bytes(value)).unwrap();
    }

    #[test]
    fn export_is_deterministic_relative_and_prefers_package_zero() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-client-globals-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_tables(&root);
        write_dat(&root, "0001/ROM/0/11.DAT", "base area");
        write_dat(&root, "R000100/ROM/0/11.DAT", "decoded area");
        write_dat(&root, "R000101/ROM/0/0.DAT", "title");
        write_dat(&root, "R000102/ROM/0/1.DAT", "status");

        let output_path = root.join("out/client-globals.json");
        export_client_globals(root.clone(), output_path.clone()).unwrap();
        let first = fs::read_to_string(&output_path).unwrap();
        export_client_globals(root.clone(), output_path.clone()).unwrap();
        let second = fs::read_to_string(&output_path).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["language"], "english");
        assert_eq!(report["resources"].as_array().unwrap().len(), 3);
        assert_eq!(report["resources"][0]["name"], "area_names");
        assert_eq!(report["resources"][0]["dat_id"], 55_465);
        assert_eq!(
            report["resources"][0]["selected_path"],
            "R000100/ROM/0/11.DAT"
        );
        assert_eq!(
            report["resources"][0]["entries"][0]["values"][0]["string"],
            "decoded area"
        );
        assert_eq!(report["resources"][1]["name"], "titles");
        assert_eq!(report["resources"][2]["name"], "status_names");
        assert!(!first.contains(&root.to_string_lossy().to_string()));
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }
}
