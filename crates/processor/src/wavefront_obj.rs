use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use dats::formats::zone_data::{grid_mesh::Point3D, zone_model::ZoneMesh};

pub fn make_wavefront_file(zone_mesh: &ZoneMesh, out_path: PathBuf) -> Result<()> {
    fs::create_dir_all(&out_path.parent().unwrap())?;
    let mut out_file = File::create(&out_path)
        .map_err(|err| anyhow!("Could not create file at {}: {}", out_path.display(), err))?;

    let mut vertices = Vec::new();
    let mut triangles = Vec::new();

    zone_mesh.mesh.grid_cells.iter().for_each(|cell| {
        cell.indices.iter().for_each(|index| {
            let block = &zone_mesh.mesh.blocks[index.block_idx as usize];
            let placement = &zone_mesh.mesh.placements[index.placement_idx as usize];
            let start_vertex = vertices.len() + 1;
            let flip_vertices = determinant(&placement.o2w) > 0f32;

            block.vertices.iter().for_each(|vertex| {
                let mut transformed_vertex = apply_matrix_to_vertex(&placement.o2w, vertex);
                transformed_vertex.y = -transformed_vertex.y;
                transformed_vertex.z = -transformed_vertex.z;
                vertices.push(transformed_vertex);
            });

            if flip_vertices {
                block.triangles.iter().for_each(|triangle| {
                    triangles.push((
                        triangle.vertex3_idx as usize + start_vertex,
                        triangle.vertex2_idx as usize + start_vertex,
                        triangle.vertex1_idx as usize + start_vertex,
                    ));
                });
            } else {
                block.triangles.iter().for_each(|triangle| {
                    triangles.push((
                        triangle.vertex1_idx as usize + start_vertex,
                        triangle.vertex2_idx as usize + start_vertex,
                        triangle.vertex3_idx as usize + start_vertex,
                    ));
                });
            }
        })
    });

    for vertex in vertices {
        out_file.write(
            format!(
                "v {} {} {}\n",
                format_f32(vertex.x),
                format_f32(vertex.y),
                format_f32(vertex.z)
            )
            .as_bytes(),
        )?;
    }

    for triangle in triangles {
        out_file.write(format!("f {} {} {}\n", triangle.0, triangle.1, triangle.2).as_bytes())?;
    }

    Ok(())
}

fn format_f32(val: f32) -> String {
    format!("{:.3}", val)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn apply_matrix_to_vertex(matrix: &[[f32; 3]; 4], vertex: &Point3D) -> Point3D {
    let x =
        matrix[0][0] * vertex.x + matrix[1][0] * vertex.y + matrix[2][0] * vertex.z + matrix[3][0];
    let y =
        matrix[0][1] * vertex.x + matrix[1][1] * vertex.y + matrix[2][1] * vertex.z + matrix[3][1];
    let z =
        matrix[0][2] * vertex.x + matrix[1][2] * vertex.y + matrix[2][2] * vertex.z + matrix[3][2];

    Point3D { x, y, z }
}

fn determinant(matrix: &[[f32; 3]; 4]) -> f32 {
    return matrix[0][0] * matrix[1][1] * matrix[2][2]
        + matrix[0][1] * matrix[1][2] * matrix[2][0]
        + matrix[0][2] * matrix[1][0] * matrix[2][1]
        - matrix[0][0] * matrix[1][2] * matrix[2][1]
        - matrix[0][1] * matrix[1][0] * matrix[2][2]
        - matrix[0][2] * matrix[1][1] * matrix[2][0];
}
