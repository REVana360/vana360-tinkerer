use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use dats::formats::zone_data::{
    ChunkData, ZoneData, math::Vertex, zone_mmb::ZoneMmbModels, zone_model::ZoneMesh,
};

pub fn make_collision_wavefront_file(zone_mesh: &ZoneMesh, out_path: PathBuf) -> Result<()> {
    fs::create_dir_all(&out_path.parent().unwrap())?;
    let mut out_file = File::create(&out_path)
        .map_err(|err| anyhow!("Could not create file at {}: {}", out_path.display(), err))?;

    let mut vertex_count: usize = 1;
    for cell in zone_mesh.mesh.grid_cells.iter() {
        for index in &cell.indices {
            let block = &zone_mesh.mesh.blocks[index.block_idx as usize];
            let placement = &zone_mesh.mesh.placements[index.placement_idx as usize];

            let flip_vertices = placement.o2w.determinant() > 0f32;

            for vertex in &block.vertices {
                let vertex = placement.o2w.apply_to_vertex(vertex);
                out_file.write(
                    format!(
                        "v {} {} {}\n",
                        format_f32(vertex.x),
                        format_f32(-vertex.y),
                        format_f32(-vertex.z)
                    )
                    .as_bytes(),
                )?;
            }

            if flip_vertices {
                for tri in &block.triangles {
                    out_file.write(
                        format!(
                            "f {} {} {}\n",
                            tri.vertex3_idx as usize + vertex_count,
                            tri.vertex2_idx as usize + vertex_count,
                            tri.vertex1_idx as usize + vertex_count
                        )
                        .as_bytes(),
                    )?;
                }
            } else {
                for tri in &block.triangles {
                    out_file.write(
                        format!(
                            "f {} {} {}\n",
                            tri.vertex1_idx as usize + vertex_count,
                            tri.vertex2_idx as usize + vertex_count,
                            tri.vertex3_idx as usize + vertex_count
                        )
                        .as_bytes(),
                    )?;
                }
            }

            vertex_count += block.vertices.len();
        }
    }

    Ok(())
}

struct MmbModel {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

struct MmbBlock {
    models: Vec<MmbModel>,
}

pub fn make_model_wavefront_file(data: &ZoneData, out_path: PathBuf) -> Result<()> {
    fs::create_dir_all(&out_path.parent().unwrap())?;
    let mut out_file = File::create(&out_path)
        .map_err(|err| anyhow!("Could not create file at {}: {}", out_path.display(), err))?;

    let mmbs = data
        .chunks
        .iter()
        .filter_map(|chunk| match &chunk.data {
            ChunkData::ZoneMmb { zone_mmb } => Some(zone_mmb),
            _ => None,
        })
        .map(|mmb| {
            let blocks = mmb
                .blocks
                .iter()
                .map(|block| {
                    let models = match &block.models {
                        ZoneMmbModels::Simple { models } => models
                            .iter()
                            .map(|model| {
                                let vertices = model
                                    .vertices
                                    .iter()
                                    .map(|vertex| Vertex {
                                        x: vertex.x,
                                        y: vertex.y,
                                        z: vertex.z,
                                    })
                                    .collect();
                                MmbModel {
                                    vertices,
                                    indices: model.indices.clone(),
                                }
                            })
                            .collect::<Vec<_>>(),
                        ZoneMmbModels::Complex { models } => models
                            .iter()
                            .map(|model| {
                                let vertices = model
                                    .vertices
                                    .iter()
                                    .map(|vertex| Vertex {
                                        x: vertex.x,
                                        y: vertex.y,
                                        z: vertex.z,
                                    })
                                    .collect();
                                MmbModel {
                                    vertices,
                                    indices: model.indices.clone(),
                                }
                            })
                            .collect::<Vec<_>>(),
                    };

                    models
                })
                .map(|models| MmbBlock { models })
                .collect::<Vec<_>>();
            (mmb.header.mmb_id.clone(), blocks)
        })
        .collect::<HashMap<_, _>>();

    let mut vertex_offset: usize = 1;
    data.chunks
        .iter()
        .filter_map(|chunk| match &chunk.data {
            ChunkData::ZoneModel { zone_model } => Some(&zone_model.model_block_instances),
            _ => None,
        })
        .flatten()
        .enumerate()
        .for_each(|(instance_idx, instance)| {
            let Some(blocks) = mmbs.get(&instance.id) else {
                eprintln!("Missing block with ID: {}", instance.id);
                return;
            };

            out_file
                .write(format!("\no {}-{}\n", instance_idx, instance.id).as_bytes())
                .unwrap();

            let o2w = instance.to_world_matrix();
            let flip_vertices = o2w.determinant() <= 0f32;

            blocks.iter().for_each(|block| {
                block.models.iter().for_each(|model| {
                    model.vertices.iter().for_each(|vertex| {
                        let vertex = o2w.apply_to_vertex(vertex);
                        out_file
                            .write(
                                format!(
                                    "v {} {} {}\n",
                                    format_f32(vertex.x),
                                    format_f32(-vertex.y),
                                    format_f32(-vertex.z)
                                )
                                .as_bytes(),
                            )
                            .unwrap();
                    });
                    model
                        .indices
                        .windows(3)
                        .enumerate()
                        .for_each(|(i, indices)| {
                            let mut v1 = indices[0] as usize + vertex_offset;
                            let v2 = indices[1] as usize + vertex_offset;
                            let mut v3 = indices[2] as usize + vertex_offset;
                            let mut do_flip = flip_vertices;
                            if i % 2 == 0 {
                                do_flip = !do_flip;
                            }
                            if do_flip {
                                std::mem::swap(&mut v1, &mut v3);
                            }
                            out_file
                                .write(format!("f {v1} {v2} {v3}\n",).as_bytes())
                                .unwrap();
                        });

                    vertex_offset += model.vertices.len();
                });
            });
        });

    Ok(())
}

fn format_f32(val: f32) -> String {
    format!("{:.3}", val)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
