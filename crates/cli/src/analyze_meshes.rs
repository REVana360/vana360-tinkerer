use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use dats::{
    context::DatContext,
    formats::zone_data::{ZoneData, zone_model::ZoneMesh},
    id_mapping::DatIdMapping,
};
use tokio::task::JoinSet;

use crate::util::get_zone_ids_from_dats;

pub async fn analyze_zone_meshes(ffxi_path: PathBuf) -> Result<()> {
    let dat_context = DatContext::from_ffxi_path(ffxi_path)?;

    let zone_infos = get_zone_ids_from_dats(&DatIdMapping::get().zone_data, &dat_context).await?;

    let mut join_set: JoinSet<Result<(u16, ZoneData)>> = JoinSet::new();

    for zone_info in zone_infos {
        let dat_context = dat_context.clone();

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

            Ok::<_, anyhow::Error>((zone_info.id, zone_data.dat))
        });
    }

    join_set
        .join_all()
        .await
        .into_iter()
        .for_each(|res| match res {
            Ok((id, zone_data)) => {
                let Ok(zone_mesh) = ZoneMesh::parse_from_zone_data(&zone_data) else {
                    eprintln!("Could not parse {id}");
                    return;
                };

                eprintln!("ID: {id}");

                if zone_mesh.wide_grid_search() {
                    eprintln!("  Has wide search!");
                }
            }
            Err(err) => {
                eprintln!("Parse error: {err:?}");
            }
        });

    Ok(())
}
