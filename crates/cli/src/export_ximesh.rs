use std::{io::Write, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use dats::{
    context::DatContext, formats::zone_data::zone_model::ZoneCollisionMesh,
    id_mapping::DatIdMapping,
};
use flate2::{write::ZlibEncoder, Compression};
use processor::ximesh::get_ximesh_bytes;
use tokio::{fs, task::JoinSet};

use crate::util::get_zone_ids_from_dats;

pub async fn export_zone_meshes(ffxi_path: PathBuf, mut out_dir: PathBuf) -> Result<()> {
    let dat_context = DatContext::from_ffxi_path(ffxi_path)?;

    let zone_infos = get_zone_ids_from_dats(&DatIdMapping::get().zone_data, &dat_context).await?;

    fs::create_dir_all(&out_dir).await?;
    out_dir = fs::canonicalize(out_dir).await?;

    let mut join_set: JoinSet<Result<()>> = JoinSet::new();

    for zone_info in zone_infos {
        let dat_context = dat_context.clone();

        let mut out_path = out_dir.clone();
        out_path.push(format!("{}.ximesh", zone_info.id));

        join_set.spawn(async move {
            eprintln!("Parsing \"{}\": \"{}\"", zone_info.id, zone_info.name);
            let zone_data_dat = DatIdMapping::get()
                .zone_data
                .get(&zone_info.id)
                .ok_or(anyhow!("No zone data for {}", zone_info.id))?;

            let zone_data = dat_context
                .get_data_from_dat(zone_data_dat)
                .with_context(|| {
                    format!(
                        "Failed to get data for \"{}\" ({})",
                        zone_info.name, zone_info.id
                    )
                })?;

            let zone_model =
                ZoneCollisionMesh::parse_from_zone_data(&zone_data.dat).with_context(|| {
                    format!(
                        "Failed to parse data for \"{}\" ({})",
                        zone_info.name, zone_info.id
                    )
                })?;

            let mesh_data = get_ximesh_bytes(zone_model);

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&mesh_data)?;
            fs::write(out_path, encoder.finish()?).await?;

            Ok::<_, anyhow::Error>(())
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                eprintln!("Parse error: {err:?}");
            }
            Err(err) => {
                eprintln!("Join error: {err:?}");
            }
        }
    }

    Ok(())
}
