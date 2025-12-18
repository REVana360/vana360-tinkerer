use std::u32;

use common::byte_walker::ByteWalker;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::formats::zone_data::{math::TransformationMatrix, math::Vertex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelBlockInstance {
    pub id: String,
    pub translation: Vertex,
    pub rotation: Vertex,
    pub scale: Vertex,
    pub unk_floats: [f32; 4],
    pub unk_vals: [u32; 8],
}

impl ModelBlockInstance {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        node_count: u32,
    ) -> Result<Vec<ModelBlockInstance>> {
        let mut nodes = Vec::with_capacity(node_count as usize);

        for _ in 0..node_count {
            let id = walker.take_str_utf8_space_end(16)?;
            let translation = Vertex::parse(walker)?;
            let rotation = Vertex::parse(walker)?;
            let scale = Vertex::parse(walker)?;

            let mut unk_floats = [0f32; 4];
            unk_floats
                .iter_mut()
                .for_each(|v| *v = walker.step().unwrap_or_default());

            let mut unk_vals = [0u32; 8];
            unk_vals
                .iter_mut()
                .for_each(|v| *v = walker.step().unwrap_or_default());

            nodes.push(ModelBlockInstance {
                id,
                translation,
                rotation,
                scale,
                unk_floats,
                unk_vals,
            });
        }
        Ok(nodes)
    }

    pub fn to_world_matrix(&self) -> TransformationMatrix {
        TransformationMatrix::from_vectors(self.translation, self.rotation, self.scale)
    }
}
