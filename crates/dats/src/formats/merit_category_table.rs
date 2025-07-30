use anyhow::{Ok, Result, anyhow};
use common::{byte_walker::ByteWalker, writing_byte_walker::WritingByteWalker};
use num_enum::{FromPrimitive, IntoPrimitive};
use serde_derive::{Deserialize, Serialize};

use crate::{dat_format::DatFormat, flags::JobFlag};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    FromPrimitive,
    IntoPrimitive,
)]
#[repr(u8)]
pub enum MeritCategory {
    None = 0,

    HpMp = 1,
    Attributes = 2,
    CombatSkills = 3,
    MagicSkills = 4,
    Others = 5,

    WarriorGroup1 = 6,
    MonkGroup1 = 7,
    WhiteMageGroup1 = 8,
    BlackMageGroup1 = 9,
    RedMageGroup1 = 10,
    ThiefGroup1 = 11,
    PaladinGroup1 = 12,
    DarkKnightGroup1 = 13,
    BeastmasterGroup1 = 14,
    BardGroup1 = 15,
    RangerGroup1 = 16,
    SamuraiGroup1 = 17,
    NinjaGroup1 = 18,
    DragoonGroup1 = 19,
    SummonerGroup1 = 20,
    BlueMageGroup1 = 21,
    CorsairGroup1 = 22,
    PuppetmasterGroup1 = 23,
    DancerGroup1 = 24,
    ScholarGroup1 = 25,

    WeaponSkills = 26,

    GeomancerGroup1 = 27,
    RuneFencerGroup1 = 28,

    WarriorGroup2 = 32,
    MonkGroup2 = 33,
    WhiteMageGroup2 = 34,
    BlackMageGroup2 = 35,
    RedMageGroup2 = 36,
    ThiefGroup2 = 37,
    PaladinGroup2 = 38,
    DarkKnightGroup2 = 39,
    BeastmasterGroup2 = 40,
    BardGroup2 = 41,
    RangerGroup2 = 42,
    SamuraiGroup2 = 43,
    NinjaGroup2 = 44,
    DragoonGroup2 = 45,
    SummonerGroup2 = 46,
    BlueMageGroup2 = 47,
    CorsairGroup2 = 48,
    PuppetmasterGroup2 = 49,
    DancerGroup2 = 50,
    ScholarGroup2 = 51,

    GeomancerGroup2 = 53,
    RuneFencerGroup2 = 54,

    #[num_enum(catch_all)]
    Unnamed(u8),
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MeritCategoryTable {
    pub categories: Vec<MeritCategoryEntry>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeritCategoryEntry {
    category: MeritCategory,
    max_merits: u16,
    jobs: JobFlag,
}

impl MeritCategoryEntry {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let category = walker.step::<u8>()?;
        walker.expect_n_msg::<u8>(0, 2, "Padding expected after first category")?;
        walker.expect_msg(category, "Category expected to repeat")?;

        let max_merits = walker.step::<u16>()?;
        walker.expect_n_msg::<u8>(0, 2, "Padding expected after max_merits field")?;

        let jobs = JobFlag::from_bits(walker.step::<u32>()?).unwrap_or_default();

        Ok(Self {
            category: MeritCategory::from_primitive(category),
            max_merits,
            jobs,
        })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write::<u8>(self.category.into());
        walker.skip(2);
        walker.write::<u8>(self.category.into());
        walker.write(self.max_merits);
        walker.skip(2);
        walker.write::<u32>(self.jobs.bits());

        Ok(())
    }
}

const ENTRY_SIZE: usize = 12;

impl MeritCategoryTable {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!(
                "Length does not match a merit category table DAT: {}",
                walker.len()
            ));
        }

        let category_count = walker.len() / ENTRY_SIZE;
        let mut categories = Vec::with_capacity(category_count);
        for _ in 0..category_count {
            let category = MeritCategoryEntry::parse(walker)?;
            categories.push(category);
        }

        Ok(MeritCategoryTable { categories })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.set_size(self.categories.len() * ENTRY_SIZE);

        for category in &self.categories {
            category.write(walker)?;
        }

        Ok(())
    }
}

impl DatFormat for MeritCategoryTable {
    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        MeritCategoryTable::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!(
                "Length does not match a merit category table DAT: {}",
                walker.len()
            ));
        }
        MeritCategoryTable::parse(walker)?;
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
        dat_format::DatFormat, flags::JobFlag, formats::merit_category_table::MeritCategory,
    };

    use super::MeritCategoryTable;

    #[test]
    pub fn merit_category_table() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/merit_category_table.DAT");

        MeritCategoryTable::check_path(&dat_path).unwrap();
        let res = MeritCategoryTable::from_path_checked_yaml(&dat_path).unwrap();

        let weapon_skills = res.categories.get(6).unwrap();
        assert_eq!(weapon_skills.category, MeritCategory::WeaponSkills);
        assert_eq!(weapon_skills.max_merits, 15);
        assert_eq!(weapon_skills.jobs, JobFlag::All);
    }
}
