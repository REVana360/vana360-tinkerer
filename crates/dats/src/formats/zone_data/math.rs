use anyhow::Result;
use common::byte_walker::ByteWalker;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vertex {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Ok(Self {
            x: walker.step()?,
            y: walker.step()?,
            z: walker.step()?,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct TransformationMatrix {
    m: [[f32; 3]; 4],
}

impl Into<TransformationMatrix> for [[f32; 3]; 4] {
    fn into(self) -> TransformationMatrix {
        TransformationMatrix { m: self }
    }
}

impl TransformationMatrix {
    pub fn apply_to_vertex(&self, vertex: &Vertex) -> Vertex {
        let m = &self.m;
        let x = m[0][0] * vertex.x + m[1][0] * vertex.y + m[2][0] * vertex.z + m[3][0];
        let y = m[0][1] * vertex.x + m[1][1] * vertex.y + m[2][1] * vertex.z + m[3][1];
        let z = m[0][2] * vertex.x + m[1][2] * vertex.y + m[2][2] * vertex.z + m[3][2];

        Vertex { x, y, z }
    }

    pub fn determinant(&self) -> f32 {
        let m = &self.m;
        return m[0][0] * m[1][1] * m[2][2]
            + m[0][1] * m[1][2] * m[2][0]
            + m[0][2] * m[1][0] * m[2][1]
            - m[0][0] * m[1][2] * m[2][1]
            - m[0][1] * m[1][0] * m[2][2]
            - m[0][2] * m[1][1] * m[2][0];
    }

    pub fn from_vectors(translation: Vertex, rotation: Vertex, scale: Vertex) -> Self {
        let cx = rotation.x.cos();
        let sx = rotation.x.sin();
        let cy = rotation.y.cos();
        let sy = rotation.y.sin();
        let cz = rotation.z.cos();
        let sz = rotation.z.sin();

        // Intermediate rotation products
        let sxsy = sx * sy;
        let cxsy = cx * sy;

        [
            [
                // Row 1: Right Vector * Scale X
                (cy * cz) * scale.x,
                (cy * sz) * scale.x,
                (-sy) * scale.x,
            ],
            [
                // Row 2: Up Vector * Scale Y
                (sxsy * cz - cx * sz) * scale.y,
                (sxsy * sz + cx * cz) * scale.y,
                (sx * cy) * scale.y,
            ],
            [
                // Row 3: Forward Vector * Scale Z
                (cxsy * cz + sx * sz) * scale.z,
                (cxsy * sz - sx * cz) * scale.z,
                (cx * cy) * scale.z,
            ],
            [
                // Row 4: Translation
                translation.x,
                translation.y,
                translation.z,
            ],
        ]
        .into()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, [f32; 3]> {
        self.m.iter()
    }

    pub fn raw(&self) -> &[[f32; 3]; 4] {
        &self.m
    }

    pub fn into_raw(&self) -> [[f32; 3]; 4] {
        self.m
    }
}
