use std::{
    collections::{HashMap, hash_map::Entry},
    u32,
};

use common::byte_walker::ByteWalker;
use serde::{Deserialize, Serialize};

use anyhow::Result;

use super::{mesh_block::MeshBlock, mesh_placement::MeshPlacement};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TriangleInfo {
    pub vertex1_idx: u16,
    pub vertex2_idx: u16,
    pub vertex3_idx: u16,
    pub normal_idx: u16,
    pub material: u8,
    pub is_invalid_triangle: bool,
    pub is_barrier: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridMesh {
    pub grid_cells: Vec<GridCell>,
    pub blocks: Vec<MeshBlock>,
    pub placements: Vec<MeshPlacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridCell {
    pub info: u32,
    pub indices: Vec<MeshEntryIndices>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedMeshEntry {
    pub flags: u16,
    pub vertices: Vec<Point3D>,
    pub normals: Vec<Point3D>,
    pub triangles: Vec<TriangleInfo>,

    pub o2w: [[f32; 3]; 4],
    pub o2w_opts: [[u16; 2]; 4],
    pub w2o: [[f32; 3]; 4],
    pub w2o_opts: [[u16; 2]; 4],
    pub unk_floats: [f32; 9],
    pub data_field_1: u32,
    pub data_field_2: u32,
    pub unk_bytes: [u8; 4],
    pub unk_1: u32,
    pub min_y: f32,
    pub max_y: f32,
    pub unk_2: u32,
}

impl NormalizedMeshEntry {
    pub fn get_map_id(&self) -> u8 {
        ((self.data_field_1 >> 0x1A & 3) << 3 | self.data_field_1 >> 3 & 7) as u8
    }

    pub fn is_roofed(&self) -> bool {
        (self.data_field_1 >> 2 & 1) > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshEntryIndices {
    pub placement_idx: u32,
    pub block_idx: u32,
}

#[derive(Default)]
pub struct MeshCollector {
    pub blocks_offset_to_idx: HashMap<u32, u32>,
    pub blocks: Vec<MeshBlock>,
    pub placements_offset_to_idx: HashMap<u32, u32>,
    pub placements: Vec<MeshPlacement>,
}

impl GridMesh {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        grid_offset: u32,
        grid_height: u16,
        grid_width: u16,
    ) -> Result<GridMesh> {
        let mut collector = MeshCollector::default();

        let mut cells: Vec<GridCell> =
            Vec::with_capacity(grid_height as usize * grid_width as usize);

        for y in 0..grid_height {
            for x in 0..grid_width {
                let cell_header_offset = grid_offset as usize
                    + ((y as usize * grid_width as usize + x as usize) * 4) as usize;

                if cell_header_offset >= walker.len() {
                    break;
                }
                let cell_offset = walker.read_at::<u32>(cell_header_offset)?;
                if cell_offset == 0 {
                    cells.push(GridCell {
                        info: 0,
                        indices: vec![],
                    });
                    continue;
                }
                if cell_offset as usize >= walker.len() {
                    cells.push(GridCell {
                        info: u32::MAX,
                        indices: vec![],
                    });
                    continue;
                }

                let cell = Self::parse_grid_cell(walker, cell_offset, &mut collector)?;
                cells.push(cell);
            }
        }

        Ok(GridMesh {
            grid_cells: cells,
            blocks: collector.blocks,
            placements: collector.placements,
        })
    }

    fn parse_grid_cell<T: ByteWalker>(
        walker: &mut T,
        cell_offset: u32,
        collector: &mut MeshCollector,
    ) -> Result<GridCell> {
        walker.goto_usize(cell_offset as usize);

        let info = walker.step::<u32>()?;

        let _skip_sometimes = info & 0x2000 > 0;
        let entry_count = info & 0x7FF;

        let mut indices = vec![];
        for _ in 0..entry_count {
            let placement_offset = walker.step::<u32>()?;
            let block_offset = walker.step::<u32>()?;

            let block_idx = match collector.blocks_offset_to_idx.entry(block_offset) {
                Entry::Occupied(occupied_entry) => *occupied_entry.get(),
                Entry::Vacant(vacant_entry) => {
                    let current_offset = walker.offset();
                    walker.goto(block_offset);
                    collector.blocks.push(MeshBlock::parse(walker)?);
                    walker.goto_usize(current_offset);
                    *vacant_entry.insert((collector.blocks.len() - 1) as u32)
                }
            };

            let placement_idx = match collector.placements_offset_to_idx.entry(placement_offset) {
                Entry::Occupied(occupied_entry) => *occupied_entry.get(),
                Entry::Vacant(vacant_entry) => {
                    let current_offset = walker.offset();
                    walker.goto(placement_offset);
                    collector.placements.push(MeshPlacement::parse(walker)?);
                    walker.goto_usize(current_offset);
                    *vacant_entry.insert((collector.placements.len() - 1) as u32)
                }
            };

            indices.push(MeshEntryIndices {
                placement_idx,
                block_idx,
            })
        }

        Ok(GridCell { info, indices })
    }
}

impl GridCell {
    pub fn should_skip(&self) -> bool {
        (self.info & 0x2000) > 0
    }
}
