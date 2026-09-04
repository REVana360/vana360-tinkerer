use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::{
    audit_dats::audit_dats, export_client_resources::export_client_resources,
    export_items::export_items, export_key_items::export_key_items,
    export_zone_entities::export_zone_entities, export_zone_events::export_zone_events,
    export_zone_text::export_zone_text,
};

const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct CatalogIndex {
    schema_version: u32,
    client_profile: &'static str,
    catalogs: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CatalogEntry {
    file: &'static str,
    schema_version: u32,
    purpose: &'static str,
}

const CATALOGS: [CatalogEntry; 7] = [
    CatalogEntry {
        file: "client-resources.json",
        schema_version: 1,
        purpose: "Global client ID and resource tables",
    },
    CatalogEntry {
        file: "dat-manifest.json",
        schema_version: 4,
        purpose: "Xbox DAT selection, availability, and format audit",
    },
    CatalogEntry {
        file: "items.json",
        schema_version: 1,
        purpose: "Localized item IDs and client metadata",
    },
    CatalogEntry {
        file: "key-items.json",
        schema_version: 1,
        purpose: "Localized key-item IDs and descriptions",
    },
    CatalogEntry {
        file: "zone-entities.json",
        schema_version: 1,
        purpose: "Per-zone full entity IDs and names",
    },
    CatalogEntry {
        file: "zone-events.json",
        schema_version: 1,
        purpose: "Per-zone event IDs, owners, data, and bytecode",
    },
    CatalogEntry {
        file: "zone-text.json",
        schema_version: 1,
        purpose: "Per-zone event and message text IDs",
    },
];

fn write_index(output_dir: &PathBuf) -> Result<()> {
    let report = CatalogIndex {
        schema_version: INDEX_SCHEMA_VERSION,
        client_profile: "july-2009-xbox",
        catalogs: CATALOGS.to_vec(),
    };
    let mut json = serde_json::to_string_pretty(&report)?;
    json.push('\n');
    fs::write(output_dir.join("catalog-index.json"), json)?;
    Ok(())
}

pub fn export_client_contract(runtime_root: PathBuf, output_dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&output_dir)?;
    export_client_resources(
        runtime_root.clone(),
        output_dir.join("client-resources.json"),
    )?;
    audit_dats(
        runtime_root.clone(),
        Some(output_dir.join("dat-manifest.json")),
        true,
    )?;
    export_items(runtime_root.clone(), output_dir.join("items.json"))?;
    export_key_items(runtime_root.clone(), output_dir.join("key-items.json"))?;
    export_zone_entities(runtime_root.clone(), output_dir.join("zone-entities.json"))?;
    export_zone_events(runtime_root.clone(), output_dir.join("zone-events.json"))?;
    export_zone_text(runtime_root, output_dir.join("zone-text.json"))?;
    write_index(&output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_index_is_complete_and_deterministic() {
        let root = std::env::temp_dir().join(format!(
            "xi-tinkerer-contract-index-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        write_index(&root).unwrap();
        let first = fs::read_to_string(root.join("catalog-index.json")).unwrap();
        write_index(&root).unwrap();
        let second = fs::read_to_string(root.join("catalog-index.json")).unwrap();
        let report: serde_json::Value = serde_json::from_str(&first).unwrap();

        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["client_profile"], "july-2009-xbox");
        assert_eq!(report["catalogs"].as_array().unwrap().len(), 7);
        assert_eq!(report["catalogs"][0]["file"], "client-resources.json");
        assert_eq!(
            report["catalogs"][4]["purpose"],
            "Per-zone full entity IDs and names"
        );
        assert_eq!(first, second);

        fs::remove_dir_all(root).unwrap();
    }
}
