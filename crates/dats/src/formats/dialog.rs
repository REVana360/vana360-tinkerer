use std::{collections::BTreeMap, mem::size_of};

use anyhow::{anyhow, Result};
use common::{byte_walker::ByteWalker, get_padding, writing_byte_walker::WritingByteWalker};
use encoding::{decoder::Decoder, encoder::Encoder};
use serde_derive::{Deserialize, Serialize};

use crate::dat_format::DatFormat;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dialog {
    pub entries: BTreeMap<u32, String>,
}

const DIALOG_MASK: u32 = 0x80808080;
const DIALOG_U8_MASK: u8 = 0x80;

impl Dialog {
    fn parse_dialog_string<T: ByteWalker>(walker: &mut T, end: u32) -> Result<String> {
        let bytes = walker
            .take_bytes(end as usize - walker.offset())?
            .into_iter()
            .map(|byte| byte ^ DIALOG_U8_MASK)
            .collect::<Vec<_>>();

        let string = Decoder::decode_dialog(&bytes)?;

        Ok(string)
    }

    fn get_header_values<T: ByteWalker>(walker: &mut T) -> Result<(u32, u32)> {
        let size_info = walker.step::<u32>()?;

        if size_info == 0 {
            return Err(anyhow!("Possible empty dialog DAT."));
        }

        let file_size = (size_info ^ 0x10000000) + 4;

        if file_size != walker.len() as u32 {
            return Err(anyhow!(
                "Invalid file size {} with byte count {}.",
                file_size,
                walker.len()
            ));
        }

        let shifted_string_count = walker.step::<u32>()? ^ DIALOG_MASK;
        if shifted_string_count % 4 != 0
            || shifted_string_count > walker.len() as u32
            || shifted_string_count < 4
        {
            return Err(anyhow!(
                "Invalid shifted string count {} with byte count {}.",
                shifted_string_count,
                walker.len()
            ));
        }

        Ok((file_size, shifted_string_count >> 2))
    }

    fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        if walker.len() == size_of::<u32>() {
            let marker = walker.step::<u32>()?;
            if marker == 0 {
                return Ok(Self::default());
            }

            return Err(anyhow!("Invalid empty dialog marker {marker:#010X}."));
        }

        let (file_size, string_count) = Self::get_header_values(walker)?;

        let mut string_ends = (0..string_count - 1)
            .into_iter()
            .map(|_| walker.step::<u32>().map(|end| (end ^ DIALOG_MASK) + 4))
            .collect::<Result<Vec<_>>>()?;

        string_ends.push(file_size);

        let result = Dialog {
            entries: string_ends
                .into_iter()
                .enumerate()
                .map(|(idx, end)| Ok((idx as u32, Self::parse_dialog_string(walker, end)?)))
                .collect::<Result<_>>()?,
        };

        Ok(result)
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        if self.entries.is_empty() {
            walker.set_size(size_of::<u32>());
            walker.write(0u32);
            return Ok(());
        }

        let encoded_strings = self
            .entries
            .iter()
            .map(|(_, string)| Encoder::encode_dialog(string))
            .collect::<Result<Vec<_>>>()?;

        // Calculate size of the DAT
        let string_lengths_header_end = 4 + encoded_strings.len() * 4;
        let mut file_size: usize = string_lengths_header_end
            + encoded_strings
                .iter()
                .map(|bytes| bytes.len())
                .sum::<usize>();

        // Add padding
        file_size += get_padding(file_size);

        walker.set_size(file_size);

        // Write header with file size and string endings
        walker.write((file_size as u32 ^ 0x10000000) - 4);
        walker.write(((encoded_strings.len() as u32) << 2) ^ DIALOG_MASK);

        // Write the ending index for each string except the last one.
        let mut encoded_strings_iter = encoded_strings.iter();
        let mut current_ending =
            string_lengths_header_end + encoded_strings_iter.next().unwrap().len() - 4;

        for encoded_string in encoded_strings_iter {
            walker.write((current_ending as u32) ^ DIALOG_MASK);
            current_ending += encoded_string.len();
        }

        // Write the strings
        for mut encoded_string in encoded_strings {
            encoded_string.iter_mut().for_each(|b| {
                *b ^= DIALOG_U8_MASK;
            });
            walker.write_bytes(&encoded_string);
        }

        for _ in 0..walker.remaining() {
            walker.write(DIALOG_U8_MASK);
        }

        Ok(())
    }
}

impl DatFormat for Dialog {
    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Dialog::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        Dialog::parse(walker)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use common::byte_walker::BufferedByteWalker;

    use crate::{dat_format::DatFormat, formats::dialog::Dialog};

    #[test]
    fn check_type_rejects_a_header_without_its_string_table() {
        let mut header_only = Vec::new();
        header_only.extend_from_slice(&0x10000004u32.to_le_bytes());
        header_only.extend_from_slice(&(8u32 ^ super::DIALOG_MASK).to_le_bytes());

        assert!(Dialog::check_type(&mut BufferedByteWalker::on(&header_only)).is_err());
    }

    #[test]
    fn empty_sentinel_round_trips() {
        let bytes = [0u8; 4];

        let dialog = Dialog::from_bytes(&bytes).unwrap();

        Dialog::check_type(&mut BufferedByteWalker::on(&bytes)).unwrap();
        assert!(dialog.entries.is_empty());
        assert_eq!(dialog.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn four_byte_nonzero_marker_is_rejected() {
        assert!(Dialog::from_bytes(&1u32.to_le_bytes()).is_err());
    }

    #[test]
    fn empty_marker_requires_exactly_four_bytes() {
        assert!(Dialog::from_bytes(&[]).is_err());
        assert!(Dialog::from_bytes(&[0u8; 3]).is_err());
        assert!(Dialog::from_bytes(&[0u8; 5]).is_err());
    }

    #[test]
    fn single_entry_round_trips() {
        let dialog = Dialog {
            entries: BTreeMap::from([(0, "Single entry.".to_string())]),
        };

        let bytes = dialog.to_bytes().unwrap();

        assert_eq!(Dialog::from_bytes(&bytes).unwrap(), dialog);
    }

    #[test]
    pub fn whitegate() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/dialog_whitegate.DAT");

        Dialog::check_path(&dat_path).unwrap();
        let res = Dialog::from_path_checked(&dat_path).unwrap();

        assert_eq!(res.entries.get(&129).unwrap(), "You observe no changes.");
    }
}
