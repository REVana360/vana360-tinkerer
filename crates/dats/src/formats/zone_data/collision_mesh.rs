use common::byte_walker::ByteWalker;
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TriangleInfo {
    pub vertex1_idx: u32,
    pub vertex2_idx: u32,
    pub vertex3_idx: u32,
    pub normal_idx: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollisionMesh {
    pub grid_entries: Vec<GridMeshEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridMeshEntry {
    pub info_entry: u32,
    pub mesh_entries: Vec<MeshEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshEntry {
    pub flags: u16,
    pub vertices: Vec<Point3D>,
    pub normals: Vec<Point3D>,
    pub triangles: Vec<TriangleInfo>,
}

#[derive(Debug)]
struct MeshEntryOffsets {
    pub vis_offset: u32,
    pub geo_offset: u32,
}

impl CollisionMesh {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        grid_offset: u32,
        grid_height: u16,
        grid_width: u16,
    ) -> Result<CollisionMesh> {
        let mut grid_entries = Vec::with_capacity(grid_height as usize * grid_width as usize);

        for y in 0..grid_height {
            for x in 0..grid_width {
                let entry_header_offset = grid_offset as usize
                    + ((y as usize * grid_width as usize + x as usize) * 4) as usize;

                if entry_header_offset >= walker.len() {
                    break;
                }
                let entry_offset = walker.read_at::<u32>(entry_header_offset)?;
                if entry_offset == 0 {
                    continue;
                }
                if entry_offset as usize >= walker.len() {
                    continue;
                }
                let grid_entry = Self::parse_grid_entry(walker, entry_offset)?;
                grid_entries.push(grid_entry);
            }
        }

        Ok(CollisionMesh { grid_entries })
    }

    fn parse_grid_entry<T: ByteWalker>(walker: &mut T, entry_offset: u32) -> Result<GridMeshEntry> {
        walker.goto_usize(entry_offset as usize);

        let info_entry = walker.step::<u32>()?;

        let mut mesh_entries = vec![];
        loop {
            let vis_offset = walker.step::<u32>()?;
            if vis_offset == 0 {
                break;
            }
            let geo_offset = walker.step::<u32>()?;
            if geo_offset == 0 {
                break;
            }
            mesh_entries.push(MeshEntryOffsets {
                vis_offset,
                geo_offset,
            })
        }

        Ok(GridMeshEntry {
            info_entry,
            mesh_entries: mesh_entries
                .into_iter()
                .map(|mesh_entry| Self::parse_grid_mesh(walker, mesh_entry))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn parse_grid_mesh<T: ByteWalker>(
        walker: &mut T,
        mesh_entry: MeshEntryOffsets,
    ) -> Result<MeshEntry> {
        let mut matrix = [[0f32; 4]; 4];

        walker.goto_usize(mesh_entry.vis_offset as usize);

        for row in matrix.iter_mut() {
            for value in row.iter_mut() {
                *value = walker.step::<f32>()?;
            }
        }

        walker.goto_usize(mesh_entry.geo_offset as usize);
        let vertex_offset = walker.step::<u32>()?;
        let normal_offset = walker.step::<u32>()?;
        let triangle_offset = walker.step::<u32>()?;
        let triangle_count = walker.step::<u16>()?;
        let flags = walker.step::<u16>()?;

        let vertex_count = (normal_offset - vertex_offset) / 12;
        let normal_count = (triangle_offset - normal_offset) / 12;

        let mut vertices = Vec::with_capacity(vertex_count as usize);
        let mut normals = Vec::with_capacity(normal_count as usize);
        let mut triangles = Vec::with_capacity(triangle_count as usize);

        walker.goto_usize(vertex_offset as usize);
        for _ in 0..vertex_count {
            let x = walker.step::<f32>()?;
            let y = walker.step::<f32>()?;
            let z = walker.step::<f32>()?;

            let vertex = Point3D {
                x: matrix[0][0] * x + matrix[1][0] * y + matrix[2][0] * z + matrix[3][0],
                y: -(matrix[0][1] * x + matrix[1][1] * y + matrix[2][1] * z + matrix[3][1]),
                z: matrix[0][2] * x + matrix[1][2] * y + matrix[2][2] * z + matrix[3][2],
            };
            vertices.push(vertex);
        }

        assert_eq!(walker.offset(), normal_offset as usize);
        for _ in 0..normal_count {
            let x = walker.step::<f32>()?;
            let y = walker.step::<f32>()?;
            let z = walker.step::<f32>()?;

            let normal = Point3D { x, y: -y, z };
            normals.push(normal);
        }

        /*
            a b c
            d e f
            g h i
        */

        let determinant = // --
            matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) //    a*(ei - fh)
            + matrix[0][1] * (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) //  b*(fg - di)
            + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]); // c*(dh - eg)

        assert_eq!(walker.offset(), triangle_offset as usize);
        for _ in 0..triangle_count {
            let v1 = (walker.step::<u16>()? & 0x3FFF) as u32;
            let v2 = (walker.step::<u16>()? & 0x3FFF) as u32;
            let v3 = (walker.step::<u16>()? & 0x3FFF) as u32;
            let n = (walker.step::<u16>()? & 0x3FFF) as u32;

            let triangle = if determinant > 0.0 {
                TriangleInfo {
                    vertex1_idx: v3,
                    vertex2_idx: v2,
                    vertex3_idx: v1,
                    normal_idx: n,
                }
            } else {
                TriangleInfo {
                    vertex1_idx: v1,
                    vertex2_idx: v2,
                    vertex3_idx: v3,
                    normal_idx: n,
                }
            };
            triangles.push(triangle);
        }

        Ok(MeshEntry {
            flags,
            vertices,
            normals,
            triangles,
        })
    }
}
