use std::{collections::BTreeSet, io::Write, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use dats::{
    context::DatContext, formats::zone_data::zone_model::ZoneCollisionMesh,
    id_mapping::DatIdMapping,
};
use flate2::{write::ZlibEncoder, Compression};
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

            let triangle_count: usize = zone_model
                .collision_mesh
                .grid_entries
                .iter()
                .map(|grid_entry| {
                    grid_entry
                        .mesh_entries
                        .iter()
                        .map(|mesh_entry| mesh_entry.triangles.len())
                })
                .flatten()
                .sum();

            let vertices = zone_model
                .collision_mesh
                .grid_entries
                .iter()
                .map(|grid_entry| {
                    grid_entry
                        .mesh_entries
                        .iter()
                        .map(|mesh_entry| {
                            mesh_entry
                                .vertices
                                .iter()
                                .map(|p| (p.x.to_le_bytes(), p.y.to_le_bytes(), p.z.to_le_bytes()))
                        })
                        .flatten()
                })
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            let vertex_count: usize = vertices.len();

            // Mesh structure is:
            // - Triangle count: u32
            // - Vertex count: u32
            // - Vertices: 3 coords per vertex * f32 per coord
            // - Indices: 3 per triangle * u32 per index
            let header_size = size_of::<u32>() + size_of::<u32>();
            let vertices_byte_len = vertex_count * 3 * size_of::<f32>();
            let indices_byte_len = triangle_count * 3 * size_of::<u32>();
            let mut mesh_data = vec![0u8; header_size + vertices_byte_len + indices_byte_len];

            let vertices_start = header_size;
            let mut vertices_offset = 0;

            let indices_start = vertices_start + vertices_byte_len;
            let mut indices_offset = 0;

            // SAFETY: mesh_data has been sized appropriately above, so no unchecked bounds should fail
            unsafe {
                // Write in triangle count
                mesh_data
                    .get_unchecked_mut(..4)
                    .copy_from_slice(&(triangle_count as u32).to_le_bytes());

                mesh_data
                    .get_unchecked_mut(4..8)
                    .copy_from_slice(&(vertex_count as u32).to_le_bytes());

                // Write all the vertices
                vertices.iter().for_each(|vertex| {
                    for coord in [vertex.0, vertex.1, vertex.2].iter() {
                        mesh_data
                            .get_unchecked_mut(
                                vertices_start + vertices_offset
                                    ..vertices_start + vertices_offset + size_of::<f32>(),
                            )
                            .copy_from_slice(coord);

                        vertices_offset += size_of::<f32>();
                    }
                });

                // Write all the indices
                zone_model
                    .collision_mesh
                    .grid_entries
                    .iter()
                    .for_each(|grid_entry| {
                        grid_entry.mesh_entries.iter().for_each(|mesh_entry| {
                            mesh_entry.triangles.iter().for_each(|triangle| {
                                for internal_vertex_idx in [
                                    triangle.vertex3_idx,
                                    triangle.vertex2_idx,
                                    triangle.vertex1_idx,
                                ] {
                                    let vertex = mesh_entry
                                        .vertices
                                        .get_unchecked(internal_vertex_idx as usize);

                                    let idx = vertices
                                        .binary_search(&(
                                            vertex.x.to_le_bytes(),
                                            vertex.y.to_le_bytes(),
                                            vertex.z.to_le_bytes(),
                                        ))
                                        .unwrap()
                                        as u32;

                                    mesh_data
                                        .get_unchecked_mut(
                                            indices_start + indices_offset
                                                ..indices_start + indices_offset + size_of::<u32>(),
                                        )
                                        .copy_from_slice(&idx.to_le_bytes());

                                    indices_offset += size_of::<u32>();
                                }
                            })
                        })
                    });
            }

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
