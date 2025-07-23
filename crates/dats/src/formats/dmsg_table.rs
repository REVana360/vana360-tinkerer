use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use common::{
    byte_walker::ByteWalker, expect, slice_byte_walker::SliceByteWalker,
    writing_byte_walker::WritingByteWalker,
};
use serde_derive::{Deserialize, Serialize};

use crate::dat_format::DatFormat;

use super::dmsg_list::DmsgEntryList;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DmsgTable {
    pub bytes_per_list: u32,
    pub flip_bytes: bool,
    pub lists: BTreeMap<u32, DmsgEntryList>,
}

#[derive(Debug)]
struct DmsgHeader {
    flip_bytes: bool,
    metadata_size: u32,
    bytes_per_list: u32,
    entry_count: u32,
}

#[derive(Debug)]
pub struct DmsgListMetaInfo {
    pub offset: u32,
    pub size: u32,
}

const HEADER_SIZE: u32 = 0x40;

impl DmsgTable {
    fn parse_header<T: ByteWalker>(walker: &mut T) -> Result<DmsgHeader> {
        walker.expect_utf8_str("d_msg\0\0\0")?;

        walker.expect(1u16)?;

        let flip_bytes_flag = walker.step::<u16>()?;
        if flip_bytes_flag != 0 && flip_bytes_flag != 1 {
            return Err(anyhow!("Unexpected flip bytes flag: {flip_bytes_flag}"));
        }
        let flip_bytes = flip_bytes_flag == 1;

        walker.expect(3u32)?;
        walker.expect(3u32)?;

        let file_bytes = walker.len() as u32;
        walker.expect(file_bytes)?;

        let header_bytes: u32 = walker.step()?;
        expect(HEADER_SIZE, header_bytes)?;

        let metadata_size: u32 = walker.step()?;

        let bytes_per_entry: u32 = walker.step()?;

        let data_size: u32 = walker.step()?;

        let entry_count: u32 = walker.step()?;

        if metadata_size > 0 && bytes_per_entry > 0 {
            return Err(anyhow!(
                "Expected only either metadata size ({}) or bytes per entry ({}) to be set.",
                metadata_size,
                bytes_per_entry,
            ));
        }

        if bytes_per_entry > 0 {
            // Each entry has a fixed size
            expect(data_size, entry_count * bytes_per_entry)?;
        }

        if metadata_size > 0 {
            // Each metadata entry should be 8 bytes in size
            expect(metadata_size, entry_count * 8)?;
        }

        expect(file_bytes, HEADER_SIZE + metadata_size + data_size)?;

        walker.expect(1u32)?;
        walker.expect(0u64)?;
        walker.expect(0u64)?;

        Ok(DmsgHeader {
            flip_bytes,
            metadata_size,
            bytes_per_list: bytes_per_entry,
            entry_count,
        })
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<DmsgTable> {
        let header: DmsgHeader = Self::parse_header(walker)?;

        let lists = if header.bytes_per_list > 0 {
            // Fixed sized entries
            (0..header.entry_count)
                .map(|idx| {
                    expect(
                        HEADER_SIZE + idx * header.bytes_per_list,
                        walker.offset() as u32,
                    )?;

                    Ok((
                        idx,
                        DmsgEntryList::parse(walker, header.flip_bytes, header.bytes_per_list)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?
        } else {
            // Variable sized entries based on metadata

            let mut metadata_walker =
                SliceByteWalker::new(walker.take_bytes(header.metadata_size as usize)?);

            let mut metadata = (0..header.entry_count)
                .map(|_| {
                    let offset = metadata_walker.step::<u32>()?;
                    let size = metadata_walker.step::<u32>()?;

                    Ok(DmsgListMetaInfo { offset, size })
                })
                .collect::<Result<Vec<_>>>()?;

            if header.flip_bytes {
                metadata.iter_mut().for_each(|info| {
                    info.offset = info.offset ^ u32::MAX;
                    info.size = info.size ^ u32::MAX;
                })
            }

            metadata
                .into_iter()
                .enumerate()
                .map(|(idx, meta)| {
                    expect(
                        HEADER_SIZE + header.metadata_size + meta.offset,
                        walker.offset() as u32,
                    )?;
                    Ok((
                        idx as u32,
                        DmsgEntryList::parse(walker, header.flip_bytes, meta.size)?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?
        };

        Ok(DmsgTable {
            bytes_per_list: header.bytes_per_list,
            flip_bytes: header.flip_bytes,
            lists,
        })
    }

    fn write_header<T: WritingByteWalker>(&self, walker: &mut T, data_size: u32) -> Result<()> {
        let list_count = self.lists.len() as u32;
        let metadata_size = if self.bytes_per_list == 0 {
            list_count * 8
        } else {
            0
        };

        walker.write_str("d_msg");
        walker.skip(3);

        walker.write(1u16);
        walker.write(if self.flip_bytes { 1u16 } else { 0u16 });

        walker.write(3u32);
        walker.write(3u32);

        // File len
        walker.write::<u32>(HEADER_SIZE + metadata_size + data_size);

        // Header len
        walker.write::<u32>(HEADER_SIZE);

        // Metadata size
        walker.write::<u32>(metadata_size);

        // Bytes per entry
        walker.write::<u32>(self.bytes_per_list);

        // Data entries
        walker.write::<u32>(data_size);

        // Amount of list entries
        walker.write::<u32>(list_count);

        walker.write(1u32);
        walker.write(0u64);
        walker.write(0u64);

        Ok(())
    }

    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        let list_count = self.lists.len() as u32;

        let mut data_size = 0;
        if self.bytes_per_list > 0 {
            // Fixed size per list

            // Skip header
            walker.goto(HEADER_SIZE);

            data_size = list_count * self.bytes_per_list;

            let mut next_end = walker.offset() + self.bytes_per_list as usize;
            for idx in 0..list_count {
                let Some(list) = self.lists.get(&idx) else {
                    walker.skip(self.bytes_per_list as usize);
                    continue;
                };

                list.write(walker, self.flip_bytes)?;

                if next_end < walker.offset() {
                    let diff = self.bytes_per_list as usize + walker.offset() - next_end;
                    return Err(anyhow!(
                        "Entry {}, can't fit with given bytes per entry ({} vs {}):\n{:#?}",
                        idx,
                        self.bytes_per_list,
                        diff,
                        list
                    ));
                }
                let diff = next_end.saturating_sub(walker.offset());
                walker.write_bytes(&vec![if self.flip_bytes { u8::MAX } else { 0u8 }; diff]);
                next_end += self.bytes_per_list as usize;
            }
        } else {
            // Variable size per list

            // Skip header and list metadata
            walker.goto(HEADER_SIZE + list_count * 8);

            let mut metas = Vec::with_capacity(list_count as usize);
            for idx in 0..list_count {
                let Some(list) = self.lists.get(&idx) else {
                    continue;
                };

                let meta = list.write(walker, self.flip_bytes)?;
                data_size += meta.size;
                metas.push(meta);
            }

            // Go back and write list metadata
            walker.goto(HEADER_SIZE);
            let mask = if self.flip_bytes { u32::MAX } else { 0 };
            let metadata_offset = HEADER_SIZE + list_count * 8;
            for meta in metas {
                walker.write((meta.offset - metadata_offset) ^ mask);
                walker.write(meta.size ^ mask);
            }
        }

        // Go back and write the header now that the total data size is known
        walker.goto(0);
        self.write_header(walker, data_size)?;

        Ok(())
    }
}

impl DatFormat for DmsgTable {
    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        DmsgTable::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        DmsgTable::parse_header(walker)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{dat_format::DatFormat, formats::dmsg_list::DmsgContent};

    use super::DmsgTable;

    #[test]
    pub fn ability_names() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/ability_names.DAT");

        DmsgTable::check_path(&dat_path).unwrap();
        let res = DmsgTable::from_path_checked(&dat_path).unwrap();

        assert_eq!(
            res.lists.get(&1).unwrap().content[0],
            DmsgContent::String {
                string: "Combo".to_string()
            }
        );

        assert_eq!(
            res.lists.get(&2).unwrap().content[0],
            DmsgContent::String {
                string: "Shoulder Tackle".to_string()
            }
        );
    }

    #[test]
    pub fn key_items() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/key_items.DAT");

        DmsgTable::check_path(&dat_path).unwrap();
        let res = DmsgTable::from_path_checked_yaml(&dat_path).unwrap();

        assert_eq!(
            res.lists.get(&1).unwrap().content[0],
            DmsgContent::Number { number: 1 }
        );
        assert_eq!(
            res.lists.get(&1).unwrap().content[4],
            DmsgContent::String {
                string: "Zeruhn report".to_string()
            }
        );

        assert_eq!(
            res.lists.get(&1534).unwrap().content[0],
            DmsgContent::Number { number: 619 }
        );
        assert_eq!(
            res.lists.get(&1534).unwrap().content[4],
            DmsgContent::String {
                string: "All-You-Can-Ride Pass".to_string()
            }
        );
        assert_eq!(
            res.lists.get(&1534).unwrap().content[6],
            DmsgContent::String {
                string: "\nFor GM use only!".to_string()
            }
        );
    }

    #[test]
    pub fn area_names() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/area_names.DAT");

        DmsgTable::check_path(&dat_path).unwrap();
        let res = DmsgTable::from_path_checked(&dat_path).unwrap();

        let psoxja = res.lists.get(&9).unwrap().content.get(0).unwrap();
        match psoxja {
            DmsgContent::String { string } => {
                assert_eq!(string, "Pso'Xja");
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    pub fn merits() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/merits.DAT");

        DmsgTable::check_path(&dat_path).unwrap();
        let res = DmsgTable::from_path_checked(&dat_path).unwrap();

        let altruism = res.lists.get(&3264).unwrap().content.get(0).unwrap();
        match altruism {
            DmsgContent::String { string } => {
                assert_eq!(string, "Altruism");
            }
            _ => {
                assert!(false);
            }
        }

        let altruism_help = res.lists.get(&3265).unwrap().content.get(0).unwrap();
        match altruism_help {
            DmsgContent::String { string } => {
                assert_eq!(
                    string,
                    "Light Arts Stratagem.\nIncreases the accuracy of your next white magic spell.\nIncrease magic accuracy by 5."
                );
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    pub fn mount_names() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/mount_names.DAT");

        DmsgTable::check_path(&dat_path).unwrap();
        let res = DmsgTable::from_path_checked_yaml(&dat_path).unwrap();

        let crawler_content = &res.lists.get(&8).unwrap().content;
        assert_eq!(crawler_content.len(), 2);

        match &crawler_content[0] {
            DmsgContent::String { string } => {
                assert_eq!(string, "Crawler");
            }
            _ => {
                assert!(false);
            }
        }

        match &crawler_content[1] {
            DmsgContent::Number { number } => {
                assert_eq!(*number, 87);
            }
            _ => {
                assert!(false);
            }
        }
    }
}
