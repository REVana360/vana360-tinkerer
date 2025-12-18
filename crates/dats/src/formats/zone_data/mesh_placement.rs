use anyhow::Result;

use common::byte_walker::ByteWalker;
use serde::{Deserialize, Serialize};

use crate::formats::zone_data::math::TransformationMatrix;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshPlacement {
    pub o2w: TransformationMatrix,
    pub o2w_opts: [[u16; 2]; 4],
    pub w2o: TransformationMatrix,
    pub w2o_opts: [[u16; 2]; 4],
    pub unk_floats: [f32; 9],
    pub data_field: u32,
    pub unk_coll4_offset: u32,
    pub unk_bytes: [u8; 4],
    pub unk_1: u32,
    pub min_y: f32,
    pub max_y: f32,
    pub unk_2: u32,
}

impl MeshPlacement {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        // Object to world transformation matrix
        let mut o2w = [[0f32; 3]; 4];
        let mut o2w_opts = [[0u16; 2]; 4];

        for (idx, row) in o2w.iter_mut().enumerate() {
            for value in row.iter_mut() {
                *value = walker.step::<f32>()?;
            }
            o2w_opts[idx][0] = walker.step::<u16>()?;
            o2w_opts[idx][1] = walker.step::<u16>()?;
        }

        // World to object transformation matrix
        let mut w2o = [[0f32; 3]; 4];
        let mut w2o_opts = [[0u16; 2]; 4];

        for (idx, row) in w2o.iter_mut().enumerate() {
            for value in row.iter_mut() {
                *value = walker.step::<f32>()?;
            }
            w2o_opts[idx][0] = walker.step::<u16>()?;
            w2o_opts[idx][1] = walker.step::<u16>()?;
        }

        // Unknown floats
        let mut unk_floats = [0f32; 9];
        for v in unk_floats.iter_mut() {
            *v = walker.step()?;
        }

        let data_field = walker.step::<u32>()?;
        let unk_coll4_offset = walker.step::<u32>()?;

        let mut unk_bytes = [0u8; 4];
        for v in unk_bytes.iter_mut() {
            *v = walker.step()?;
        }

        let unk_1 = walker.step::<u32>()?;
        let min_y = walker.step::<f32>()?;
        let max_y = walker.step::<f32>()?;
        let unk_2 = walker.step::<u32>()?;

        Ok(Self {
            o2w: o2w.into(),
            o2w_opts,
            w2o: w2o.into(),
            w2o_opts,
            unk_floats,
            data_field,
            unk_coll4_offset,
            unk_bytes,
            unk_1,
            min_y,
            max_y,
            unk_2,
        })
    }

    pub fn get_map_id(&self) -> u8 {
        ((self.data_field >> 0x1A & 3) << 3 | self.data_field >> 3 & 7) as u8
    }

    pub fn is_roofed(&self) -> bool {
        (self.data_field >> 2 & 1) > 0
    }
}
