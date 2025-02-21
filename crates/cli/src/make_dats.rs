use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{mpsc, Arc},
};

use anyhow::{anyhow, Result};
use dats::context::{DatContext, ZoneName};
use processor::{
    dat_descriptor::DatDescriptor,
    processor::{DatProcessingState, DatProcessor},
};
use project::{DAT_GENERATION_DIR, LOOKUP_TABLE_DIR, RAW_DATA_DIR, ZONE_MAPPING_FILE};

pub fn make_dats(
    project_path: PathBuf,
    yaml_paths: &[PathBuf],
    out_dir: Option<PathBuf>,
) -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let mut processor = DatProcessor::new(tx);

    println!("Processing project: {}", project_path.display());

    let lookup_dir = project_path.join(LOOKUP_TABLE_DIR);

    // Load zone mapping
    let zone_map_file = lookup_dir.join(ZONE_MAPPING_FILE);
    let zone_file = File::open(zone_map_file)
        .map_err(|err| anyhow!("Unable to open zone mapping file: {}", err))?;
    let zones_mapping: HashMap<u16, ZoneName> = serde_yaml::from_reader(zone_file)
        .map_err(|err| anyhow!("Unable to read zone mapping file: {}", err))?;

    let dat_context = Arc::new(DatContext::from_path_and_zone_mappings(
        lookup_dir,
        zones_mapping,
    )?);

    let in_dir = project_path.join(RAW_DATA_DIR);
    let out_dir = out_dir.unwrap_or_else(|| project_path.join(DAT_GENERATION_DIR));

    let mut total_count = 0;
    if yaml_paths.is_empty() {
        // Generate all yaml files in project directory
        total_count = processor.all_yaml_to_dats(dat_context, &in_dir, &out_dir);
    } else {
        // Try to generate from specified yaml files
        let dat_descriptors = yaml_paths.iter().filter_map(|path| {
            match DatDescriptor::from_path(&path, &in_dir, &dat_context) {
                Some(descriptor) => Some(descriptor),
                None => {
                    eprintln!(
                        "Was not able to load or determine DAT for the yaml file: {}",
                        path.display()
                    );
                    None
                }
            }
        });

        for dat_descriptor in dat_descriptors {
            processor.yaml_to_dat(
                dat_descriptor,
                dat_context.clone(),
                in_dir.clone(),
                out_dir.clone(),
            );
            total_count += 1;
        }
    }

    println!("Generating {} DATs", total_count);

    let mut finished = 0;
    while finished < total_count {
        let msg = rx.recv()?;
        match msg.state {
            DatProcessingState::Working => {}
            DatProcessingState::Finished(_) => {
                finished += 1;
            }
            DatProcessingState::Error(err) => {
                return Err(anyhow!("Processing error: {}", err));
            }
        }
    }

    println!("Done");

    Ok(())
}
