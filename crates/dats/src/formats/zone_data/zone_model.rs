use anyhow::{Result, anyhow};
use common::byte_walker::ByteWalker;
use serde_derive::{Deserialize, Serialize};

use crate::{formats::zone_data::model_blocks::ModelBlockInstance, serde_hex};

use super::{ChunkData, ZoneData, grid_mesh::GridMesh};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMesh {
    pub data_len: u32,

    pub len_and_type: u32,
    pub model_block_instance_count: u32,
    pub model_block_unk: u8,
    pub model_block_instances: Vec<ModelBlockInstance>,
    pub mesh_header_offset: u32,

    pub grid_width: u16,
    pub grid_height: u16,
    pub cell_width: u8,
    pub cell_height: u8,

    pub unk_coll1_offset: u32,
    pub unk_coll2_end_offset: u32,
    pub unk_coll3_offset: u32,

    pub flags: u32,

    #[serde(with = "serde_hex")]
    pub misc_data: Vec<u8>,

    pub blocks_count: u32,
    pub blocks_offset: u32,
    pub unk_coll4_count: u32,
    pub unk_coll4_offset: u32,
    pub grid_offset: u32,
    pub placements_offset: u32,
    pub placements_count: u32,

    pub mesh: GridMesh,
}

impl ZoneMesh {
    pub fn parse_from_zone_data(zone_data: &ZoneData) -> Result<&ZoneMesh> {
        zone_data
            .chunks
            .iter()
            .find_map(|chunk| match &chunk.data {
                ChunkData::ZoneModel { zone_model } => Some(zone_model),
                _ => None,
            })
            .ok_or_else(|| anyhow!("No zone model found in zone data."))
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneMesh> {
        let data_len = walker.len() as u32;

        let len_and_type = walker.step::<u32>()?; // 0x00
        let node_count_and_unk = walker.step::<u32>()?; // 0x04
        let model_block_instance_count = node_count_and_unk & 0xFFFFFF;
        let model_block_unk = (node_count_and_unk >> 24) as u8;
        let mesh_header_offset = walker.step::<u32>()?; // 0x08

        if data_len <= mesh_header_offset {
            return Err(anyhow!(
                "Invalid mesh offset found: {mesh_header_offset}, expecting less than {data_len}"
            ));
        }

        let num_cells_width = walker.step::<u8>()?; // 0x0C
        let num_cells_height = walker.step::<u8>()?; // 0x0D
        let cell_width = walker.step::<u8>()?; // 0x0E
        let cell_height = walker.step::<u8>()?; // 0x0F

        let temp_width = num_cells_width as i64 * cell_width as i64;
        let temp_height = num_cells_height as i64 * cell_height as i64;

        let grid_width = ((temp_width + (temp_width >> 31 & 3)) >> 2) as u16;
        let grid_height = ((temp_height + (temp_height >> 31 & 3)) >> 2) as u16;

        let unk_coll1_offset = walker.step::<u32>()?; // 0x10

        let unk_coll2_offset = 0x20;
        let unk_coll2_end_offset = walker.step::<u32>()?; // 0x14
        let _unk_coll2_count = (unk_coll2_end_offset.saturating_sub(unk_coll2_offset)) / 0x64;

        let unk_coll3_offset = walker.step::<u32>()?; // 0x18
        let _unk_coll3_count = (mesh_header_offset.saturating_sub(unk_coll3_offset)) / 0x4C;

        let flags = walker.step::<u32>()?; // 0x1C

        let model_block_instances = ModelBlockInstance::parse(walker, model_block_instance_count)?;

        if (mesh_header_offset as usize) < walker.offset() {
            return Err(anyhow!(
                "Invalid mesh header offset found: {mesh_header_offset}, expected greater than {}",
                walker.offset()
            ));
        }

        let misc_data = walker
            .take_bytes(mesh_header_offset as usize - walker.offset())?
            .to_vec();

        let blocks_count = walker.step::<u32>()?;
        let blocks_offset = walker.step::<u32>()?;

        let unk_coll4_count = walker.step::<u32>()?;
        let unk_coll4_offset = walker.step::<u32>()?;

        let grid_offset = walker.step::<u32>()?;

        let placements_offset = walker.step::<u32>()?;
        let placements_count = walker.step::<u32>()?;

        let _placement_entry_size: usize = if walker.read_at::<u8>(3).unwrap_or_default() < 0x1A {
            0x80
        } else {
            0xC0
        };

        let mesh = GridMesh::parse(walker, grid_offset, grid_height, grid_width)?;

        Ok(ZoneMesh {
            data_len,

            len_and_type,
            model_block_instance_count,
            model_block_unk,
            model_block_instances,

            mesh_header_offset,

            grid_width,
            grid_height,
            cell_width,
            cell_height,

            unk_coll1_offset,
            unk_coll2_end_offset,
            unk_coll3_offset,

            flags,

            misc_data,

            blocks_count,
            blocks_offset,
            unk_coll4_count,
            unk_coll4_offset,
            grid_offset,
            placements_offset,
            placements_count,

            mesh,
        })
    }

    pub fn wide_grid_search(&self) -> bool {
        (self.flags.to_le_bytes()[1] >> 3 & 1) > 0
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{dat_format::DatFormat, formats::zone_data::ZoneData};

    use super::ZoneMesh;

    #[test]
    pub fn zone_model() {
        let resources_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // let names: &[&str] = &["Pashhow_Marshlands", "Yughott_Grotto"];
        // let names: &[&str] = &["Korroloka_Tunnel"];
        let names: &[&str] = &["Sea_Serpent_Grotto"];

        for name in names {
            let mut path = resources_path.clone();
            path.push(format!("resources/test/zone_data_{name}.DAT"));

            let data = ZoneData::from_path(&path).unwrap();
            let _res = ZoneMesh::parse_from_zone_data(&data).unwrap();

            // let file = File::create(format!("{name}.yml")).unwrap();
            // serde_yaml::to_writer(BufWriter::new(file), &res).unwrap();
        }
    }
}
