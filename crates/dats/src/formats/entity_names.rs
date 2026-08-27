use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use common::{byte_walker::ByteWalker, writing_byte_walker::WritingByteWalker};
use encoding::{decoder::Decoder, encoder::Encoder};
use serde_derive::{Deserialize, Serialize};

use crate::dat_format::DatFormat;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityNames {
    pub names: Vec<EntityName>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityName {
    id: u32,
    name: String,
}

impl EntityName {
    pub const fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn get_entity_names_zone(path: &PathBuf) -> Option<u16> {
    let mut file = File::open(path).ok()?;

    let mut four_bytes = [0u8; 4];
    file.read_exact(&mut four_bytes).ok()?;

    let starts_with_none = four_bytes == "none".as_bytes();
    if !starts_with_none {
        return None;
    }

    let mut first_id = 0;
    let mut current_record = 0;
    while first_id == 0 {
        file.seek(SeekFrom::Start(0x1C + current_record * 0x1C + 0x18))
            .ok()?;
        file.read_exact(&mut four_bytes).ok()?;

        first_id = u32::from_le_bytes(four_bytes);
        current_record += 1;
    }

    Some(((first_id >> 12) & 0xFFF) as u16)
}

impl EntityNames {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<EntityNames> {
        walker.expect_utf8_str("none")?;
        if walker.remaining() < 24 || (walker.remaining() - 24) % 28 != 0 {
            return Err(anyhow!(
                "Entity-name DAT must contain a 28-byte header and complete 28-byte records."
            ));
        }

        Ok(EntityNames {
            names: EntityNames::read_names(walker)?,
        })
    }

    fn read_names<T: ByteWalker>(walker: &mut T) -> Result<Vec<EntityName>> {
        walker.goto(28);

        let mut names = vec![];
        while walker.remaining() >= 28 {
            names.push(parse_next_entity_name(walker)?);
        }

        Ok(names)
    }
}

fn parse_next_entity_name<T: ByteWalker>(walker: &mut T) -> Result<EntityName> {
    let name = Decoder::decode_simple(walker.take_bytes(24)?)?;
    let id: u32 = walker.step()?;

    Ok(EntityName { id, name })
}

impl DatFormat for EntityNames {
    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write_bytes("none".as_bytes());
        walker.set_size(28);
        walker.goto(28);

        for name in self.names.iter() {
            let name_bytes = Encoder::encode_simple(&name.name)?;
            if name_bytes.len() > 24 {
                return Err(anyhow!(
                    "Name can at most be 24 bytes long: '{}'",
                    name.name
                ));
            }

            walker.write_bytes(&name_bytes);
            walker.skip(24 - name_bytes.len());
            walker.write(name.id);
        }

        Ok(())
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        EntityNames::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        walker.expect_utf8_str("none")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bytes() -> Vec<u8> {
        let mut bytes = b"none".to_vec();
        bytes.resize(28, 0);
        bytes.extend_from_slice(b"Ceraul");
        bytes.resize(52, 0);
        bytes.extend_from_slice(&0x010E6001u32.to_le_bytes());
        bytes.extend_from_slice(b"Aubejart");
        bytes.resize(80, 0);
        bytes.extend_from_slice(&0x010E6002u32.to_le_bytes());
        bytes
    }

    #[test]
    fn parses_and_writes_28_byte_records() {
        let bytes = sample_bytes();
        let entities = EntityNames::from_bytes_checked(&bytes).unwrap();

        assert_eq!(entities.names.len(), 2);
        assert_eq!(entities.names[0].name(), "Ceraul");
        assert_eq!(entities.names[0].id(), 0x010E6001);
        assert_eq!(entities.names[1].name(), "Aubejart");
        assert_eq!(entities.names[1].id(), 0x010E6002);
    }

    #[test]
    fn empty_header_has_no_records() {
        let mut bytes = b"none".to_vec();
        bytes.resize(28, 0);

        let entities = EntityNames::from_bytes_checked(&bytes).unwrap();

        assert!(entities.names.is_empty());
    }

    #[test]
    fn rejects_incomplete_header_or_record() {
        assert!(EntityNames::from_bytes(b"none").is_err());

        let mut bytes = b"none".to_vec();
        bytes.resize(29, 0);
        assert!(EntityNames::from_bytes(&bytes).is_err());
    }
}
