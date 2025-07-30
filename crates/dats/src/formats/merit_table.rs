use std::collections::{BTreeMap, HashMap};

use anyhow::{Ok, Result, anyhow};
use common::{byte_walker::ByteWalker, expect_msg, writing_byte_walker::WritingByteWalker};
use num_enum::FromPrimitive;
use serde_derive::{Deserialize, Serialize};

use crate::{
    dat_format::DatFormat, flags::JobFlag, formats::merit_category_table::MeritCategory, serde_hex,
};

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MeritTable {
    pub category_count: u8,
    pub categories: BTreeMap<MeritCategory, MeritCategoryEntries>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MeritCategoryEntry {
    Entry {
        /// This index relates to offset of the name and description in the DmsgTable DAT for merits.
        /// The name is found at (category * 64) + (entry_idx * 2), and the description is found right after it.
        entry_idx: u8,
        max_merits: u8,

        #[serde(with = "serde_hex")]
        unknown: Vec<u8>,
        jobs: JobFlag,
    },

    /// It seems like SE has left in some non-matching indices for certain entries.
    /// In order to fully replicate the original DAT, it is saved in the exported data.
    UnexpectedEntryIdx { entry_idx: u8, unexpected_idx: u8 },
}

impl MeritCategoryEntry {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        category_sizes: &mut HashMap<u8, u8>,
        expected_full_idx: u16,
        expected_category: u8,
    ) -> Result<Option<Self>> {
        walker.expect::<u16>(expected_full_idx)?;

        let category = walker.step::<u8>()?;
        if category != 0 {
            expect_msg(expected_category, category, "Unexpected merit category")?;
        }

        let entry_idx = walker.step::<u8>()?;
        let max_merits = walker.step::<u8>()?;

        let unknown = walker.take_bytes(3)?.to_vec();
        let jobs = JobFlag::from_bits(walker.step::<u32>()?).unwrap_or_default();

        // The entry idx for each category keeps counting up.
        // For the 0 category, it ends up wrapping around.
        let expected_entry_idx = category_sizes
            .entry(expected_category)
            .and_modify(|x| {
                *x = x.wrapping_add(1u8);
            })
            .or_insert(1u8)
            .wrapping_sub(1u8);

        if max_merits == 0 && category == 0 && unknown.iter().all(|v| v == &0) {
            if entry_idx != expected_entry_idx {
                // See comment on [MeritCategoryEntry::UnexpectedEntryIdx]
                return Ok(Some(Self::UnexpectedEntryIdx {
                    entry_idx: expected_entry_idx,
                    unexpected_idx: entry_idx,
                }));
            }

            return Ok(None);
        }

        if entry_idx != expected_entry_idx {
            return Err(anyhow!(
                "Expected entry idx to be {}, but got {} for category {}",
                expected_entry_idx,
                entry_idx,
                expected_category,
            ));
        }

        Ok(Some(Self::Entry {
            entry_idx,
            max_merits,
            unknown,
            jobs,
        }))
    }

    pub fn write<T: WritingByteWalker>(
        &self,
        walker: &mut T,
        full_idx: u16,
        category: u8,
    ) -> Result<()> {
        walker.write::<u16>(full_idx);

        match self {
            MeritCategoryEntry::Entry {
                entry_idx,
                max_merits,
                unknown,
                jobs,
            } => {
                if unknown.len() != 3 {
                    return Err(anyhow!(
                        "Expected exactly 3 unknown bytes, but found {} at category {} and entry {}",
                        unknown.len(),
                        category,
                        entry_idx
                    ));
                }

                walker.write::<u8>(category);
                walker.write::<u8>(*entry_idx);
                walker.write::<u8>(*max_merits);
                walker.write_bytes(&unknown);
                walker.write(jobs.bits());
            }
            MeritCategoryEntry::UnexpectedEntryIdx {
                entry_idx: _,
                unexpected_idx,
            } => {
                walker.write::<u8>(0);
                walker.write::<u8>(*unexpected_idx);
                walker.write::<u8>(0);
                walker.skip(3);
                walker.write::<u32>(0);
            }
        }

        Ok(())
    }
}

const ENTRY_SIZE: usize = 0x180;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeritCategoryEntries(Vec<MeritCategoryEntry>);

impl MeritCategoryEntries {
    pub fn parse<T: ByteWalker>(
        walker: &mut T,
        category_sizes: &mut HashMap<u8, u8>,
        expected_category: u8,
        expected_start_idx: u16,
    ) -> Result<Self> {
        let entries = (0..32)
            .into_iter()
            .map(|idx| {
                MeritCategoryEntry::parse(
                    walker,
                    category_sizes,
                    expected_start_idx + idx,
                    expected_category,
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|v| v)
            .collect();

        Ok(Self(entries))
    }
}

impl MeritTable {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let mut category_sizes = HashMap::new();

        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!(
                "Length does not match a merit table DAT: {}",
                walker.len()
            ));
        }

        let category_count = walker.len() / ENTRY_SIZE;
        let mut categories = BTreeMap::new();
        for category_idx in 0..category_count {
            // After a specific number of categories, the rest seems to be just padding
            let expected_category = if category_idx >= 64 {
                0u8
            } else {
                category_idx as u8
            };

            let category = MeritCategoryEntries::parse(
                walker,
                &mut category_sizes,
                expected_category,
                category_idx as u16 * 32,
            )?;

            if !category.0.is_empty() {
                categories.insert(MeritCategory::from_primitive(expected_category), category);
            }
        }

        Ok(MeritTable {
            category_count: category_count as u8,
            categories,
        })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.set_size(self.category_count as usize * ENTRY_SIZE);

        // Regular categories
        for category_idx in 0..64u8 {
            let start_idx = category_idx as u16 * 32u16;

            match self
                .categories
                .get(&MeritCategory::from_primitive(category_idx))
            {
                Some(category) => {
                    let info_map: BTreeMap<u8, &MeritCategoryEntry> = category
                        .0
                        .iter()
                        .map(|e| match e {
                            MeritCategoryEntry::Entry { entry_idx, .. } => (*entry_idx, e),
                            MeritCategoryEntry::UnexpectedEntryIdx { entry_idx, .. } => {
                                (*entry_idx, e)
                            }
                        })
                        .collect();

                    for entry_idx in 0..32u8 {
                        match info_map.get(&entry_idx) {
                            Some(entry) => {
                                entry.write(walker, start_idx + entry_idx as u16, category_idx)?
                            }
                            None => MeritCategoryEntry::Entry {
                                entry_idx: entry_idx,
                                max_merits: 0,
                                unknown: vec![0, 0, 0],
                                jobs: JobFlag::empty(),
                            }
                            .write(
                                walker,
                                start_idx + entry_idx as u16,
                                0u8,
                            )?,
                        };
                    }
                }
                None => {
                    // Blank category
                    for entry_idx in 0..32u8 {
                        MeritCategoryEntry::Entry {
                            entry_idx: entry_idx,
                            max_merits: 0,
                            unknown: vec![0, 0, 0],
                            jobs: JobFlag::empty(),
                        }
                        .write(walker, start_idx + entry_idx as u16, 0)?;
                    }
                }
            }
        }

        // Padding categories
        for category_idx in 64..self.category_count {
            let start_idx = category_idx as u16 * 32u16;
            let expected_start_idx = (category_idx - 63).wrapping_mul(32);

            for entry_idx in 0..32u8 {
                MeritCategoryEntry::Entry {
                    entry_idx: expected_start_idx + entry_idx,
                    max_merits: 0,
                    unknown: vec![0, 0, 0],
                    jobs: JobFlag::empty(),
                }
                .write(walker, start_idx + entry_idx as u16, 0u8)?;
            }
        }

        Ok(())
    }
}

impl DatFormat for MeritTable {
    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        MeritTable::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!(
                "Length does not match a merit table DAT: {}",
                walker.len()
            ));
        }
        MeritTable::parse(walker)?;
        Ok(())
    }

    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        dat_format::DatFormat,
        flags::JobFlag,
        formats::{merit_category_table::MeritCategory, merit_table::MeritCategoryEntry},
    };

    use super::MeritTable;

    #[test]
    pub fn merit_table() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/merit_table.DAT");

        MeritTable::check_path(&dat_path).unwrap();
        let res = MeritTable::from_path_checked(&dat_path).unwrap();

        let whm_group2 = res.categories.get(&MeritCategory::WhiteMageGroup2).unwrap();

        let merits = &whm_group2.0;
        assert_eq!(merits.len(), 4);

        // New merits added, so it's skipping old indices
        let expected_entry_indices = [0, 1, 4, 5];

        for (idx, expected_entry_idx) in expected_entry_indices.into_iter().enumerate() {
            let merit = merits.get(idx).unwrap();
            match merit {
                MeritCategoryEntry::Entry {
                    entry_idx,
                    max_merits,
                    unknown,
                    jobs,
                } => {
                    assert_eq!(*entry_idx, expected_entry_idx);
                    assert_eq!(*max_merits, 5);
                    assert_eq!(unknown, &vec![0, 0, 5]);
                    assert_eq!(*jobs, JobFlag::WHM);
                }
                MeritCategoryEntry::UnexpectedEntryIdx { .. } => {
                    assert!(false, "Expected a merit entry")
                }
            }
        }
    }
}
