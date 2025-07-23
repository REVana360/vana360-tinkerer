use crate::serde_hex;
use anyhow::{Ok, Result, anyhow};
use common::{
    byte_walker::ByteWalker, get_padding, slice_byte_walker::SliceByteWalker,
    writing_byte_walker::WritingByteWalker,
};
use serde_derive::{Deserialize, Serialize};

use crate::dat_format::DatFormat;

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventBlock {
    entity_id: u32,
    events: Vec<Event>,
    data: Vec<u32>,
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Event {
    id: u16,

    #[serde(with = "serde_hex")]
    byte_code: Vec<u8>,
}

impl EventBlock {
    pub fn parse<T: ByteWalker>(walker: &mut T, block_size: u32) -> Result<Self> {
        let mut walker = SliceByteWalker::new(walker.take_bytes(block_size as usize)?);

        let entity_id = walker.step::<u32>()?;

        let event_count = walker.step::<u32>()?;
        let byte_code_offsets = (0..event_count)
            .map(|_| walker.step::<u16>())
            .collect::<Result<Vec<_>, _>>()?;

        let event_ids = (0..event_count)
            .map(|_| walker.step::<u16>())
            .collect::<Result<Vec<_>, _>>()?;

        let data_count = walker.step::<u32>()?;
        let data = (0..data_count)
            .map(|_| walker.step::<u32>())
            .collect::<Result<Vec<_>, _>>()?;

        let byte_code_size = walker.step::<u32>()?;

        let events_byte_code = byte_code_offsets
            .iter()
            .zip(
                byte_code_offsets
                    .iter()
                    .skip(1)
                    .chain(std::iter::once(&(byte_code_size as u16))),
            )
            .map(|(start, end)| *end - *start)
            .map(|size| Ok(walker.take_bytes(size as usize)?.to_vec()))
            .collect::<Result<Vec<_>>>()?;

        let events = event_ids
            .into_iter()
            .zip(events_byte_code.into_iter())
            .map(|(id, byte_code)| Event { id, byte_code })
            .collect();

        Ok(Self {
            entity_id,
            events,
            data,
        })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write::<u32>(self.entity_id);

        walker.write::<u32>(self.events.len() as u32);

        // Byte code offsets
        let mut current_size: u16 = 0;
        for event in &self.events {
            walker.write::<u16>(current_size);
            current_size += event.byte_code.len() as u16;
        }

        for event in &self.events {
            walker.write::<u16>(event.id);
        }

        walker.write::<u32>(self.data.len() as u32);
        for data in &self.data {
            walker.write::<u32>(*data);
        }

        // Full size of byte code data (without padding)
        walker.write::<u32>(current_size as u32);

        // Byte code bytes
        for event in &self.events {
            walker.write_bytes(&event.byte_code);
        }

        let padding = get_padding(current_size as usize);
        for _ in 0..padding {
            walker.write::<u8>(0xFF);
        }

        Ok(())
    }

    fn calc_size(&self) -> u32 {
        let byte_code_size = self
            .events
            .iter()
            .map(|event| event.byte_code.len())
            .sum::<usize>();

        let size = size_of::<u32>() // Entity ID
        + size_of::<u32>() // Events length
        + (size_of::<u16>() + size_of::<u16>()) * self.events.len() // Event offsets and IDs
        + size_of::<u32>() // Data length
        + size_of::<u32>() * self.data.len() // Data
        + size_of::<u32>() // Full byte code length
        + byte_code_size
        + get_padding(byte_code_size);

        size as u32
    }
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Events {
    blocks: Vec<EventBlock>,
}

impl Events {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let block_count = walker.step::<u32>()?;

        if block_count > u32::MAX / 4 - 4 {
            return Err(anyhow!("Too many blocks to be an event DAT."));
        }

        let header_size = (block_count * 4 + 4) as usize;
        if walker.len() < header_size {
            return Err(anyhow!("Not enough bytes to read event block sizes."));
        }

        let mut full_size = header_size as u32;
        let block_sizes: Vec<u32> = (0..block_count)
            .map(|_| {
                let size = walker.step::<u32>()?;
                match full_size.checked_add(size) {
                    Some(new_size) => {
                        full_size = new_size;
                    }
                    None => {
                        return Err(anyhow!("Event block size is too big and has overflowed."));
                    }
                }
                if full_size > walker.len() as u32 {
                    return Err(anyhow!("Event block sizes exceed bytes in DAT."));
                }
                Ok(size)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut blocks = Vec::with_capacity(block_count as usize);
        for block_size in block_sizes {
            blocks.push(EventBlock::parse(walker, block_size)?);
        }

        Ok(Events { blocks })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write::<u32>(self.blocks.len() as u32);

        for block in &self.blocks {
            walker.write::<u32>(block.calc_size());
        }

        for block in &self.blocks {
            block.write(walker)?;
        }

        Ok(())
    }
}

impl DatFormat for Events {
    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Events::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        Events::parse(walker)?;

        Ok(())
    }

    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::dat_format::DatFormat;

    use super::Events;

    #[test]
    pub fn whitegate() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let out_path = dat_path.join("resources/test/output/events_whitegate.yml");
        dat_path.push("resources/test/events_whitegate.DAT");

        Events::check_path(&dat_path).unwrap();
        let res = Events::from_path(&dat_path).unwrap();

        fs::create_dir_all(out_path.parent().unwrap()).unwrap();
        let file = fs::File::create(out_path).unwrap();
        serde_yaml::to_writer(file, &res).unwrap();
    }
}
