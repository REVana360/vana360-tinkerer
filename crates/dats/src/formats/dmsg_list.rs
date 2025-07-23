use std::cmp::min;

use anyhow::{anyhow, Result};
use common::{byte_walker::ByteWalker, get_padding, writing_byte_walker::WritingByteWalker};
use encoding::{decoder::Decoder, encoder::Encoder};
use serde_derive::{Deserialize, Serialize};

use crate::formats::dmsg_table::DmsgListMetaInfo;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DmsgEntryList {
    pub content: Vec<DmsgContent>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DmsgContent {
    String { string: String },
    Number { number: u32 },
}

#[derive(Debug)]
pub struct DmsgListMetadata {
    pub list_offset: u32,
    pub list_length: u32,
}

#[derive(Debug)]
pub struct DmsgEntryMetaInfo {
    pub offset: u32,
    pub numbers: u32,
}

const LIST_STRING_PADDING: u32 = 28;

impl DmsgEntryList {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        flip_bytes: bool,
        list_bytes: u32,
    ) -> Result<DmsgEntryList> {
        let mask_u32 = if flip_bytes { u32::MAX } else { 0 };
        let mask_u8 = if flip_bytes { u8::MAX } else { 0 };

        let list_start_offset = walker.offset() as u32;

        let list_entry_count = walker.step::<u32>()? ^ mask_u32;

        let metas = (0..list_entry_count)
            .into_iter()
            .map(|_| {
                let offset = walker.step::<u32>()? ^ mask_u32;
                let numbers = walker.step::<u32>()? ^ mask_u32;
                if numbers == 0 && offset + LIST_STRING_PADDING + 4 > list_bytes {
                    return Err(anyhow!(
                        "Invalid offset {} for list length {}.",
                        offset,
                        list_bytes
                    ));
                }

                Ok(DmsgEntryMetaInfo { offset, numbers })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut list = DmsgEntryList {
            content: Vec::with_capacity(list_entry_count as usize),
        };

        // Read the actual strings
        for (idx, meta) in metas.into_iter().enumerate() {
            // Validate we're at the expected string offset based on metadata
            let current_string_offset = walker.offset() as u32 - list_start_offset;

            if meta.offset != current_string_offset {
                let bytes_inbetween = walker
                    .read_bytes_at(
                        (list_start_offset + min(meta.offset, current_string_offset)) as usize,
                        (current_string_offset.abs_diff(meta.offset)) as usize,
                    )?
                    .iter()
                    .map(|b| b ^ mask_u8)
                    .collect::<Vec<_>>();

                return Err(anyhow!(
                    "Invalid string offset for {}: {} vs {}\nBytes in-between: {:02X?}",
                    idx,
                    meta.offset,
                    current_string_offset,
                    bytes_inbetween
                ));
            }

            if meta.numbers > 0 {
                // Parse as number if the metadata indicates it.
                let number = walker.step::<u32>()? ^ mask_u32;

                list.content.push(DmsgContent::Number { number });
                continue;
            }

            // Initial string padding
            walker.expect_msg::<u8>(0x01 ^ mask_u8, "Indication of string start")?;
            walker.expect_n_msg::<u8>(
                mask_u8,
                LIST_STRING_PADDING as usize - 1,
                "Zero-padding before string",
            )?;

            // Read the string ending at a 0x00 (flipped to 0xFF in MASK_U8)
            let text_bytes: Vec<u8> = walker
                .step_until(mask_u8)?
                .into_iter()
                .map(|byte| byte ^ mask_u8)
                .collect();

            let string = Decoder::decode_simple(&text_bytes)?;
            walker.expect_msg::<u8>(mask_u8, "End of string")?;

            // Alignment padding
            let padding = get_padding(text_bytes.len() + 1);
            walker.expect_n_msg::<u8>(mask_u8, padding, "Alignment padding")?;

            list.content.push(DmsgContent::String { string });
        }

        let expected_end_offset = (list_start_offset + list_bytes) as usize;
        if walker.offset() < expected_end_offset {
            walker.goto(expected_end_offset as u32);
        } else if walker.offset() > expected_end_offset {
            return Err(anyhow!("Parsed past end of string list."));
        }

        Ok(list)
    }

    pub fn write<T: WritingByteWalker>(
        &self,
        walker: &mut T,
        flip_bytes: bool,
    ) -> Result<DmsgListMetaInfo> {
        let mask_u32 = if flip_bytes { u32::MAX } else { 0 };
        let mask_u8 = if flip_bytes { u8::MAX } else { 0 };

        let list_start_offset = walker.offset() as u32;

        // String count
        walker.write(self.content.len() as u32 ^ mask_u32);

        let encoded_strings = self
            .content
            .iter()
            .map(|entry| {
                match entry {
                    DmsgContent::String { string } => {
                        let mut bytes = Encoder::encode_simple(&string)?;
                        bytes.push(0x00); // End of string

                        // Add padding
                        let padding = get_padding(bytes.len());
                        for _ in 0..padding {
                            bytes.push(0x00);
                        }

                        // Flip the bytes
                        bytes.iter_mut().for_each(|byte| *byte ^= mask_u8);

                        Ok((0u32, bytes))
                    }
                    DmsgContent::Number { number } => {
                        Ok((1, (number ^ mask_u32).to_le_bytes().to_vec()))
                    }
                }
            })
            .collect::<Result<Vec<_>>>()?;

        // Write the string offsets and flags
        let meta_bytes_len = self.content.len() as u32 * 8 + 4;

        let mut current_string_offset = meta_bytes_len;
        for (flag, encoded_string) in &encoded_strings {
            walker.write::<u32>(current_string_offset ^ mask_u32);
            walker.write::<u32>(flag ^ mask_u32);

            current_string_offset += encoded_string.len() as u32;
            if *flag == 0 {
                current_string_offset += LIST_STRING_PADDING;
            }
        }

        // Write the strings
        for (flag, encoded_string) in &encoded_strings {
            if *flag == 0 {
                // String prefix
                walker.write(0x01 ^ mask_u8);
                for _ in 0..LIST_STRING_PADDING - 1 {
                    walker.write(mask_u8);
                }
            }

            walker.write_bytes(&encoded_string);
        }

        Ok(DmsgListMetaInfo {
            offset: list_start_offset,
            size: walker.offset() as u32 - list_start_offset,
        })
    }
}
