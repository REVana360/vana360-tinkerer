use anyhow::{Context, Result};
use common::byte_walker::ByteWalker;
use serde_derive::{Deserialize, Serialize};

use crate::serde_hex_num;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmb {
    pub header: ZoneMmbHeader,
    pub blocks: Vec<ZoneMmbBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlock {
    pub header: ZoneMmbBlockHeader,
    pub models: ZoneMmbModels,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum ZoneMmbModels {
    Simple {
        models: Vec<ZoneMmbModel<ZoneMmbBlockVertex>>,
    },
    Complex {
        models: Vec<ZoneMmbModel<ZoneMmbBlockVertexExtra>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbHeader {
    pub len: u32,
    #[serde(with = "serde_hex_num")]
    pub byte_0x3: u8,

    pub kind: u8,
    #[serde(with = "serde_hex_num")]
    pub byte_0x5: u8,
    #[serde(with = "serde_hex_num")]
    pub byte_0x6: u8,
    #[serde(with = "serde_hex_num")]
    pub byte_0x7: u8,

    pub moniker1: String,
    pub moniker2: String,

    pub img_id: String,
    pub pieces: u32,
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlockHeader {
    pub model_count: u32,
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub z1: f32,
    pub z2: f32,
    pub face_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbModelHeader {
    pub texture_kind: String,
    pub texture_id: String,
    pub flag1: bool,
    pub flag2: bool,
    pub flag3: bool,
    pub flag4: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlDrawType {
    TriangleList,
    TriangleStrip,
    Line,
    LineLoop,
    Polygon,
}

pub trait VertexParse
where
    Self: Sized,
{
    fn parse_vertex<T: ByteWalker>(walker: &mut T) -> Result<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlockVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
    pub color: u32,
    pub u: f32,
    pub v: f32,
}

impl VertexParse for ZoneMmbBlockVertex {
    fn parse_vertex<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Ok(ZoneMmbBlockVertex {
            x: walker.step()?,
            y: walker.step()?,
            z: walker.step()?,
            hx: walker.step()?,
            hy: walker.step()?,
            hz: walker.step()?,
            color: walker.step()?,
            u: walker.step()?,
            v: walker.step()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlockVertexExtra {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
    pub hx: f32,
    pub hy: f32,
    pub hz: f32,
    pub color: u32,
    pub u: f32,
    pub v: f32,
}

impl VertexParse for ZoneMmbBlockVertexExtra {
    fn parse_vertex<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Ok(ZoneMmbBlockVertexExtra {
            x: walker.step()?,
            y: walker.step()?,
            z: walker.step()?,
            dx: walker.step()?,
            dy: walker.step()?,
            dz: walker.step()?,
            hx: walker.step()?,
            hy: walker.step()?,
            hz: walker.step()?,
            color: walker.step()?,
            u: walker.step()?,
            v: walker.step()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbModel<VertexKind> {
    pub header: ZoneMmbModelHeader,
    pub vertices: Vec<VertexKind>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbEntry {
    pub draw_type: GlDrawType,
    pub texture_name: String,
    pub vertex_count: u16,
    pub blending: u16,
    pub vertices: Vec<ZoneMmbBlockVertex>,
    pub index_count: u32,
    pub indices: Vec<u16>,
}

impl ZoneMmb {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmb> {
        let start_offset = walker.offset();
        let header = Self::parse_header(walker)?;

        let block_offsets = Self::parse_offsets(walker, &header)?;

        let mut blocks = Vec::with_capacity(block_offsets.len());

        for block_offset in block_offsets {
            walker
                .expect_offset(start_offset + block_offset as usize)
                .with_context(|| "At next block offset")?;

            let block_header = Self::parse_block_header(walker)?;
            // if block_header.model_count > 50 {
            //     return Err(anyhow!(
            //         "Corrupt MMB model counts: {}",
            //         block_header.model_count
            //     ));
            // }

            let models =
                Self::parse_models(walker, header.kind, block_header.model_count as usize)?;

            blocks.push(ZoneMmbBlock {
                header: block_header,
                models,
            });
        }

        Ok(ZoneMmb { header, blocks })
    }

    fn parse_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbHeader> {
        let first_u32 = walker.step::<u32>()?;
        let len = first_u32 & 0xFFFFFF;
        let byte_0x3 = (first_u32 >> 24 & 0xFF) as u8;

        Ok(ZoneMmbHeader {
            len,
            byte_0x3,
            kind: walker.step::<u8>()?,
            byte_0x5: walker.step::<u8>()?,
            byte_0x6: walker.step::<u8>()?,
            byte_0x7: walker.step::<u8>()?,
            moniker1: walker.take_str_utf8_nul_end(4)?,
            moniker2: walker.take_str_utf8_nul_end(4)?,
            img_id: walker.take_str_utf8_space_end(16)?,
            pieces: walker.step()?,
            x1: walker.step()?,
            x2: walker.step()?,
            y1: walker.step()?,
            y2: walker.step()?,
            z1: walker.step()?,
            z2: walker.step()?,
        })
    }

    fn parse_models<T: ByteWalker>(
        walker: &mut T,
        kind: u8,
        model_count: usize,
    ) -> Result<ZoneMmbModels> {
        if kind != 2 {
            Ok(ZoneMmbModels::Simple {
                models: (0..model_count)
                    .map(|_| Self::parse_model::<ZoneMmbBlockVertex, _>(walker))
                    .collect::<Result<Vec<_>>>()?,
            })
        } else {
            Ok(ZoneMmbModels::Complex {
                models: (0..model_count)
                    .map(|_| Self::parse_model::<ZoneMmbBlockVertexExtra, _>(walker))
                    .collect::<Result<Vec<_>>>()?,
            })
        }
    }

    fn parse_model<VertexKind: VertexParse, T: ByteWalker>(
        walker: &mut T,
    ) -> Result<ZoneMmbModel<VertexKind>> {
        let (vertex_count, header) = Self::parse_model_header(walker)?;

        let block_start_offset = walker.offset() as u32;
        let vertex_block_size = vertex_count * size_of::<VertexKind>() as u32;

        let idx_info_offset = block_start_offset + vertex_block_size;

        let vertices = (0..vertex_count)
            .map(|_| VertexKind::parse_vertex(walker))
            .collect::<Result<Vec<_>>>()?;

        walker
            .expect_offset(idx_info_offset as usize)
            .with_context(|| "At index info offset")?;
        let vertex_idx_count = walker.step::<u32>()?;

        // let _face_count = if kind == 1 || kind == 3 {
        //     vertex_idx_count - 2
        // } else {
        //     vertex_idx_count / 3
        // };

        let indices = (0..vertex_idx_count)
            .map(|_| walker.step::<u16>())
            .collect::<Result<Vec<_>>>()?;

        let next_start = idx_info_offset + 4 + ((vertex_idx_count + 1) & 0xFFFFFFFE) * 2;

        let alignment_count = walker.offset() & 3; // mod 4
        for _ in 0..alignment_count {
            walker.step::<u8>()?;
        }

        walker
            .expect_offset(next_start as usize)
            .with_context(|| "At index next model offset")?;

        Ok(ZoneMmbModel {
            header,
            vertices,
            indices,
        })
    }

    fn parse_block_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbBlockHeader> {
        Ok(ZoneMmbBlockHeader {
            model_count: walker.step()?,
            x1: walker.step()?,
            x2: walker.step()?,
            y1: walker.step()?,
            y2: walker.step()?,
            z1: walker.step()?,
            z2: walker.step()?,
            face_id: walker.step()?,
        })
    }

    fn parse_model_header<T: ByteWalker>(walker: &mut T) -> Result<(u32, ZoneMmbModelHeader)> {
        let texture_kind = walker.take_str_utf8_space_end(8)?;
        let texture_id = walker.take_str_utf8_space_end(8)?;
        let count_and_flags = walker.step::<u32>()?;
        let vertex_count = count_and_flags & 0xFFFFFFF;
        Ok((
            vertex_count,
            ZoneMmbModelHeader {
                texture_kind,
                texture_id,
                flag1: count_and_flags >> 31 == 1,
                flag2: ((count_and_flags >> 30) & 1) == 1,
                flag3: ((count_and_flags >> 29) & 1) == 1,
                flag4: ((count_and_flags >> 28) & 1) == 1,
            },
        ))
    }

    // fn parse_block_vertex<T: ByteWalker, const WITH_D: bool>(
    //     walker: &mut T,
    // ) -> Result<ZoneMmbBlockVertex> {
    //     Ok(ZoneMmbBlockVertex {
    //         x: walker.step()?,
    //         y: walker.step()?,
    //         z: walker.step()?,
    //         dx: if WITH_D { walker.step()? } else { 0.0 },
    //         dy: if WITH_D { walker.step()? } else { 0.0 },
    //         dz: if WITH_D { walker.step()? } else { 0.0 },
    //         hx: walker.step()?,
    //         hy: walker.step()?,
    //         hz: walker.step()?,
    //         color: walker.step()?,
    //         u: walker.step()?,
    //         v: walker.step()?,
    //     })
    // }

    fn parse_offsets<T: ByteWalker>(walker: &mut T, header: &ZoneMmbHeader) -> Result<Vec<u32>> {
        let mut offsets = vec![];
        for _ in 0..header.pieces {
            let offset = walker.step::<u32>()?;
            if offset != 0 {
                offsets.push(offset)
            }
        }
        Ok(offsets)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{dat_format::DatFormat, formats::zone_data::ZoneData};

    #[test]
    pub fn zone_mmb() {
        let resources_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let names = ["Pashhow_Marshlands", "Yughott_Grotto"];

        for name in names {
            let mut path = resources_path.clone();
            path.push(format!("resources/test/zone_data_{name}.DAT"));

            let _data = ZoneData::from_path(&path).unwrap();

            // let file = File::create(format!("{name}.yml")).unwrap();
            // serde_yaml::to_writer(BufWriter::new(file), &data).unwrap();
        }
    }
}
