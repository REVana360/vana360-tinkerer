use anyhow::Result;
use common::byte_walker::ByteWalker;
use serde::{Deserialize, Serialize};

use crate::formats::zone_data::{grid_mesh::TriangleInfo, math::Vertex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshBlock {
    pub flags: u16,
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<TriangleInfo>,
    pub normals: Vec<Vertex>,
}

impl MeshBlock {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
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

        assert_eq!(vertex_offset as usize, walker.offset());
        for _ in 0..vertex_count {
            let x = walker.step::<f32>()?;
            let y = walker.step::<f32>()?;
            let z = walker.step::<f32>()?;

            let vertex = Vertex { x, y, z };
            vertices.push(vertex);
        }

        assert_eq!(walker.offset(), normal_offset as usize);
        for _ in 0..normal_count {
            let x = -walker.step::<f32>()?;
            let y = -walker.step::<f32>()?;
            let z = -walker.step::<f32>()?;

            let normal = Vertex { x, y, z };
            normals.push(normal);
        }

        assert_eq!(walker.offset(), triangle_offset as usize);
        for _ in 0..triangle_count {
            let v1 = walker.step::<u16>()?;
            let v2 = walker.step::<u16>()?;
            let v3 = walker.step::<u16>()?;
            let n = walker.step::<u16>()?;

            let vertex1_idx = v1 & 0x7FFF;
            let vertex2_idx = v2 & 0x3FFF;
            let vertex3_idx = v3 & 0x3FFF;
            let normal_idx = n & 0x7FFF;

            let material = ((((n >> 15) * 2 | (v3 >> 15)) * 2 | (v2 >> 15)) * 2 | (v1 >> 15)) as u8;

            // This bit is set during DAT loading in the client when the triangle has two vertices ontop of eachother
            let is_invalid_triangle = (v2 & 0x4000) > 0;

            let is_barrier = (v3 & 0x4000) > 0;

            let triangle = TriangleInfo {
                vertex1_idx,
                vertex2_idx,
                vertex3_idx,
                normal_idx,
                material,
                is_invalid_triangle,
                is_barrier,
            };
            triangles.push(triangle);
        }

        Ok(Self {
            flags,
            vertices,
            triangles,
            normals,
        })
    }
}
