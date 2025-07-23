pub mod collision_mesh;
pub mod zone_mmb;
pub mod zone_model;

use anyhow::{Result, anyhow};
use common::{
    byte_walker::ByteWalker, vec_byte_walker::VecByteWalker, writing_byte_walker::WritingByteWalker,
};
use encoding::chunk_key_tables::KEY_TABLE_1;
use serde_derive::{Deserialize, Serialize};
use zone_mmb::ZoneMmb;
use zone_model::ZoneCollisionMesh;

use crate::{dat_format::DatFormat, serde_hex, serde_hex_num};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ZoneData {
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub four_char_code: String,

    #[serde(with = "serde_hex_num")]
    pub chunk_type: u8,

    #[serde(with = "serde_hex_num")]
    pub unknown_0x08: u32,

    #[serde(with = "serde_hex_num")]
    pub unknown_0x12: u32,

    pub data: ChunkData,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ChunkData {
    ZoneMmb {
        zone_mmb: ZoneMmb,
    },
    ZoneModel {
        zone_model: ZoneCollisionMesh,
    },
    Unknown {
        #[serde(with = "serde_hex")]
        data: Vec<u8>,
    },
}

impl ZoneData {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneData> {
        let mut chunks = vec![];

        while walker.remaining() > 0 {
            let chunk = Chunk::parse(walker)?;
            chunks.push(chunk);
        }

        Ok(ZoneData { chunks })
    }
}

impl Chunk {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Chunk> {
        let four_char_code = std::str::from_utf8(walker.take_bytes(4)?)?.to_string();

        let type_and_length = walker.step::<u32>()? & 0xFFFFFFF;
        let unknown_0x08 = walker.step::<u32>()?;
        let unknown_0x12 = walker.step::<u32>()?;

        let chunk_type = (type_and_length & 0x7F) as u8;
        let length = (((type_and_length >> 7) & 0x7FFFF) << 4)
            .checked_sub(0x10)
            .ok_or_else(|| anyhow!("Invalid chunk data."))?;

        let data = ChunkData::parse(walker, chunk_type, length)?;

        Ok(Chunk {
            four_char_code,
            chunk_type,
            unknown_0x08,
            unknown_0x12,
            data,
        })
    }
}

impl ChunkData {
    pub fn parse<T: ByteWalker>(walker: &mut T, chunk_type: u8, length: u32) -> Result<ChunkData> {
        let data = walker.take_bytes(length as usize)?;

        if length < 8 {
            return Ok(ChunkData::Unknown { data: data.into() });
        }

        match chunk_type {
            // MZB
            0x1C => {
                Self::decode_1b(data.to_vec())
                    .and_then(|decoded| ZoneCollisionMesh::parse(&mut VecByteWalker::on(decoded)))
                    .map(|zone_model| ChunkData::ZoneModel { zone_model })
                    .or_else(|_err|  Ok(ChunkData::Unknown { data: data.into() }))
            }

            // MMB
            // 0x2E => {
            //     Self::decode_05(data.to_vec())
            //         .and_then(|decoded| Self::decode_ffff(decoded))
            //         .and_then(|decoded| ZoneMmb::parse(&mut VecByteWalker::on(decoded)))
            //         .map(|zone_mmb| ChunkData::ZoneMmb { zone_mmb })
            //         .or_else(|err| {
            //             eprintln!("Failed to parse MMB: {err:?}");
            //             Ok(ChunkData::Unknown { data: data.into() })
            //         })
            // }

            // Notes from other projects:
            0x20 | // IMG
            0x29 | // Bone
            0x2B | // Animation
            0x2A | // Vertex

            _ => Ok(ChunkData::Unknown { data: data.into() }),
        }
    }

    fn decode_1b(data: Vec<u8>) -> Result<Vec<u8>> {
        if data[3] != 0x1B {
            return Ok(data);
        }

        let mut walker = VecByteWalker::on(data);
        let len = walker.read_at::<u32>(0)? & 0xFFFFFF;

        let index = walker.read_at::<u8>(7)? ^ 0xFF;

        let mut key = KEY_TABLE_1[index as usize] as usize;

        let mut key_counter = 0;
        let mut pos = 8;
        while pos < len {
            let xor_len = ((key >> 4) & 7) + 0x10;

            if key & 1 == 1 && pos + (xor_len as u32) < len {
                for i in 0..xor_len {
                    let idx = (pos + i as u32) as usize;
                    let new_value = walker.read_at::<u8>(idx)? ^ 0xFF;
                    walker.write_at::<u8>(idx, new_value);
                }
            }
            key_counter += 1;
            key += key_counter;
            pos += xor_len as u32;
        }

        let node_count = (walker.read_at::<u32>(4)? & 0xFFFFFF) as usize;

        for i in 0..node_count {
            for j in 0..0x10 {
                let idx = (0x20 + i * 0x64 + j) as usize;
                let new_value = walker.read_at::<u8>(idx)? ^ 0x55;
                walker.write_at::<u8>(idx, new_value);
            }
        }

        Ok(walker.into_vec())
    }

    fn decode_05(data: Vec<u8>) -> Result<Vec<u8>> {
        if data[3] < 0x05 {
            return Ok(data);
        }

        let mut walker = VecByteWalker::on(data);
        let len = walker.read_at::<u32>(0)? & 0xFFFFFF;

        let index = walker.read_at::<u8>(5)? ^ 0xF0;
        let mut key = KEY_TABLE_1[index as usize] as usize;

        let mut key_counter = 0;
        for pos in 8..len {
            let x = (key << 8) | key;
            key_counter += 1;
            key += key_counter;

            let new_value = walker.read_at::<u8>(pos as usize)? ^ ((x >> (key & 0x7)) as u8);
            walker.write_at::<u8>(pos as usize, new_value);

            key_counter += 1;
            key += key_counter;
            key &= 0xFF;
        }

        Ok(walker.into_vec())
    }

    fn decode_ffff(data: Vec<u8>) -> Result<Vec<u8>> {
        if data[6] != 0xFF || data[7] != 0xFF {
            return Ok(data);
        }

        let mut walker = VecByteWalker::on(data);
        let len = walker.read_at::<u32>(0)? & 0xFFFFFF;

        let mut key1 = (walker.read_at::<u8>(5)? ^ 0xF0) as usize;
        let mut key2 = KEY_TABLE_1[key1 as usize] as usize;

        let decode_count = ((len - 8) & (!0xFu32)) / 2;

        let mut offset1: usize = 8;
        let mut offset2: usize = offset1 + decode_count as usize;
        if offset2 >= walker.len() {
            return Err(anyhow!(
                "Invalid decode_count of {} when length is {}",
                decode_count,
                walker.len()
            ));
        }

        for _ in (0..decode_count).step_by(8) {
            if (key2 & 1) == 1 {
                walker.swap_8_bytes(offset1, offset2);
            }

            key1 += 9;
            key2 += key1;
            offset1 += 8;
            offset2 += 8;
        }

        Ok(walker.into_vec())
    }
}

impl DatFormat for ZoneData {
    fn write<T: WritingByteWalker>(&self, _walker: &mut T) -> Result<()> {
        Err(anyhow!("Can't write DAT"))
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        ZoneData::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        ZoneData::parse(walker)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::dat_format::DatFormat;

    use super::ZoneData;

    #[test]
    pub fn zone_data() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/zone_data_Pashhow_Marshlands.DAT");

        ZoneData::check_path(&dat_path).unwrap();
        let _res = ZoneData::from_path(&dat_path).unwrap();
    }
}
