use anyhow::{Ok, Result, anyhow};
use common::{
    byte_walker::{BufferedByteWalker, ByteWalker},
    get_padding,
    vec_byte_walker::VecByteWalker,
    writing_byte_walker::WritingByteWalker,
};
use encoding::{decoder::Decoder, encoder::Encoder};
use serde_derive::{Deserialize, Serialize};

use crate::{
    dat_format::DatFormat,
    enums::{Element, EnglishArticle, ItemType, PuppetSlot, SkillType},
    flags::{EquipmentSlot, ItemFlag, JobFlag, Race, ValidTargets},
    serde_base64,
    utils::{get_nibble, rotate_all},
};

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ItemInfo {
    id: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    strings: Option<ItemStrings>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    raw_strings: Vec<EncodedStringBytes>,

    flags: ItemFlag,
    stack_size: u16,
    item_type: ItemType,
    resource_id: u16,
    valid_targets: ValidTargets,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    equipment: Option<EquipmentData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    weapon: Option<WeaponData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    puppet: Option<PuppetItemData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    instinct: Option<InstinctData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    furnishing: Option<FurnishingData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    usable_item: Option<UsableItemData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    currency: Option<CurrencyData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    slip: Option<SlipData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    monipulator: Option<MonipulatorData>,

    #[serde(with = "serde_base64")]
    icon_bytes: Vec<u8>,

    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    unterminated_icon_padding: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct EncodedStringBytes {
    #[serde(with = "serde_base64")]
    bytes: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStrings {
    #[serde(untagged)]
    English {
        name: String,
        article_type: EnglishArticle,
        singular_name: String,
        plural_name: String,
        description: String,
    },

    #[serde(untagged)]
    Japanese { name: String, description: String },

    #[serde(untagged)]
    Name { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemTextData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_type_code: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub singular_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plural_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemEntry {
    pub data: ItemData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ItemTextData>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemData {
    pub id: u32,
    pub flags_bits: u16,
    pub stack_size: u16,
    pub item_type_code: u16,
    pub resource_id: u16,
    pub valid_targets_bits: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equipment: Option<ItemEquipmentData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapon: Option<ItemWeaponData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub puppet: Option<ItemPuppetData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instinct: Option<ItemInstinctData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub furnishing: Option<ItemFurnishingData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usable: Option<ItemUsableData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<ItemCurrencyData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slip: Option<ItemSlipData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monipulator: Option<ItemMonipulatorData>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemEquipmentData {
    pub level: u16,
    pub slots_bits: u16,
    pub races_bits: u16,
    pub jobs_bits: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superior_level: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shield_size: Option<u16>,
    pub max_charges: u8,
    pub casting_time: u8,
    pub use_delay: u16,
    pub reuse_delay: u32,
    pub unknown1: u16,
    pub ilevel: u8,
    pub unknown2: u8,
    pub unknown3: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemWeaponData {
    pub damage: u16,
    pub delay: u16,
    pub dps: u16,
    pub skill_type_code: u8,
    pub jug_size: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown1: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemPuppetData {
    pub slot_code: u16,
    pub element_charge: u32,
    pub unknown1: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemInstinctData {
    pub unknown1: u32,
    pub unknown2: u32,
    pub unknown3: u16,
    pub instinct_cost: u16,
    pub unknown4: u16,
    pub unknown5: u32,
    pub unknown6: u32,
    pub unknown7: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemFurnishingData {
    pub element_code: u16,
    pub storage_slots: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown3: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemUsableData {
    pub activation_time: u16,
    pub unknown1: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown3: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemCurrencyData {
    pub unknown1: u16,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemSlipData {
    pub unknown1: u16,
    pub unknowns: [u32; 17],
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ItemMonipulatorData {
    pub unknown1: u16,
    pub unknowns: [u32; 24],
}

#[derive(Debug, Clone)]
pub enum ItemStringContent {
    Number(u32),
    StringBytes(Vec<u8>),
}

impl ItemStringContent {
    pub fn from_string(str: &str) -> Result<Self> {
        Self::from_bytes(&Encoder::encode_simple(str)?)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut string_walker = VecByteWalker::with_size(28);

        // Start of string and initial padding
        string_walker.write::<u32>(1);
        for _ in 0..6 {
            string_walker.write::<u32>(0);
        }

        string_walker.write_bytes(bytes);
        string_walker.write::<u8>(0); // End of string

        // Alignment padding
        let padding = get_padding(string_walker.offset());
        for _ in 0..padding {
            string_walker.write::<u8>(0);
        }
        Ok(ItemStringContent::StringBytes(string_walker.into_vec()))
    }

    pub fn from_article(article: impl Into<u32>) -> Self {
        ItemStringContent::Number(article.into())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemCategory {
    Unknown,
    Currency,
    Item,
    Armor,
    Weapon,
    PuppetItem,
    UsableItem,
    Slip,
    Instinct,
    Monipulator,
}

impl ItemCategory {
    pub fn from_id(id: u32) -> Self {
        match id {
            0xFFFF => ItemCategory::Currency,
            0..=0xFFF => ItemCategory::Item,
            0x1000..=0x1FFF => ItemCategory::UsableItem,
            0x2000..=0x21FF => ItemCategory::PuppetItem,
            0x2200..=0x27FF => ItemCategory::Item,
            0x2800..=0x3FFF => ItemCategory::Armor,
            0x4000..=0x59FF => ItemCategory::Weapon,
            0x5A00..=0x6FFF => ItemCategory::Armor,
            0x7000..=0x73FF => ItemCategory::Slip,
            0x7400..=0x77FF => ItemCategory::Instinct,
            0x7800..=0xF1FF => ItemCategory::Monipulator,
            0xF200.. => ItemCategory::Item,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentData {
    level: u16,
    slots: EquipmentSlot,
    races: Race,
    jobs: JobFlag,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    superior_level: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    shield_size: Option<u16>,

    max_charges: u8,
    casting_time: u8,
    use_delay: u16,
    reuse_delay: u32,
    unknown1: u16,
    ilevel: u8,
    unknown2: u8,
    unknown3: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponData {
    damage: u16,
    delay: u16,
    dps: u16,
    skill_type: SkillType,
    jug_size: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    unknown1: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuppetItemData {
    slot: PuppetSlot,
    element_charge: ElementValues,
    unknown1: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementValues {
    fire: u8,
    ice: u8,
    wind: u8,
    earth: u8,
    lightning: u8,
    water: u8,
    light: u8,
    dark: u8,
}

impl From<u32> for ElementValues {
    fn from(value: u32) -> Self {
        ElementValues {
            fire: get_nibble(value, 0),
            ice: get_nibble(value, 1),
            wind: get_nibble(value, 2),
            earth: get_nibble(value, 3),
            lightning: get_nibble(value, 4),
            water: get_nibble(value, 5),
            light: get_nibble(value, 6),
            dark: get_nibble(value, 7),
        }
    }
}

impl From<ElementValues> for u32 {
    fn from(value: ElementValues) -> Self {
        value.fire as u32
            + ((value.ice as u32) << (4 * 1))
            + ((value.wind as u32) << (4 * 2))
            + ((value.earth as u32) << (4 * 3))
            + ((value.lightning as u32) << (4 * 4))
            + ((value.water as u32) << (4 * 5))
            + ((value.light as u32) << (4 * 6))
            + ((value.dark as u32) << (4 * 7))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstinctData {
    unknown1: u32,
    unknown2: u32,
    unknown3: u16,
    instinct_cost: u16,
    unknown4: u16,
    unknown5: u32,
    unknown6: u32,
    unknown7: u32,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FurnishingData {
    element: Element,
    storage_slots: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    unknown3: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsableItemData {
    activation_time: u16,
    unknown1: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    unknown2: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    unknown3: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyData {
    unknown1: u16,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlipData {
    unknown1: u16,
    unknowns: [u32; 17],
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonipulatorData {
    unknown1: u16,
    unknowns: [u32; 24],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemLayout {
    Legacy,
    Modern,
}

struct ParsedItemStrings {
    strings: Option<ItemStrings>,
    raw_strings: Vec<EncodedStringBytes>,
}

impl ItemInfo {
    fn text(&self) -> Option<ItemTextData> {
        self.strings.as_ref().map(|strings| match strings {
            ItemStrings::English {
                name,
                article_type,
                singular_name,
                plural_name,
                description,
            } => ItemTextData {
                name: name.clone(),
                article_type_code: Some((*article_type).into()),
                singular_name: Some(singular_name.clone()),
                plural_name: Some(plural_name.clone()),
                description: Some(description.clone()),
            },
            ItemStrings::Japanese { name, description } => ItemTextData {
                name: name.clone(),
                article_type_code: None,
                singular_name: None,
                plural_name: None,
                description: Some(description.clone()),
            },
            ItemStrings::Name { name } => ItemTextData {
                name: name.clone(),
                article_type_code: None,
                singular_name: None,
                plural_name: None,
                description: None,
            },
        })
    }

    fn data(&self) -> ItemData {
        ItemData {
            id: self.id,
            flags_bits: self.flags.bits(),
            stack_size: self.stack_size,
            item_type_code: self.item_type.into(),
            resource_id: self.resource_id,
            valid_targets_bits: self.valid_targets.bits(),
            equipment: self.equipment.as_ref().map(|equipment| ItemEquipmentData {
                level: equipment.level,
                slots_bits: equipment.slots.bits(),
                races_bits: equipment.races.bits(),
                jobs_bits: equipment.jobs.bits(),
                superior_level: equipment.superior_level,
                shield_size: equipment.shield_size,
                max_charges: equipment.max_charges,
                casting_time: equipment.casting_time,
                use_delay: equipment.use_delay,
                reuse_delay: equipment.reuse_delay,
                unknown1: equipment.unknown1,
                ilevel: equipment.ilevel,
                unknown2: equipment.unknown2,
                unknown3: equipment.unknown3,
            }),
            weapon: self.weapon.as_ref().map(|weapon| ItemWeaponData {
                damage: weapon.damage,
                delay: weapon.delay,
                dps: weapon.dps,
                skill_type_code: weapon.skill_type.into(),
                jug_size: weapon.jug_size,
                unknown1: weapon.unknown1,
            }),
            puppet: self.puppet.as_ref().map(|puppet| ItemPuppetData {
                slot_code: puppet.slot.into(),
                element_charge: puppet.element_charge.into(),
                unknown1: puppet.unknown1,
            }),
            instinct: self.instinct.as_ref().map(|instinct| ItemInstinctData {
                unknown1: instinct.unknown1,
                unknown2: instinct.unknown2,
                unknown3: instinct.unknown3,
                instinct_cost: instinct.instinct_cost,
                unknown4: instinct.unknown4,
                unknown5: instinct.unknown5,
                unknown6: instinct.unknown6,
                unknown7: instinct.unknown7,
            }),
            furnishing: self
                .furnishing
                .as_ref()
                .map(|furnishing| ItemFurnishingData {
                    element_code: furnishing.element.into(),
                    storage_slots: furnishing.storage_slots,
                    unknown3: furnishing.unknown3,
                }),
            usable: self.usable_item.as_ref().map(|usable| ItemUsableData {
                activation_time: usable.activation_time,
                unknown1: usable.unknown1,
                unknown2: usable.unknown2,
                unknown3: usable.unknown3,
            }),
            currency: self.currency.as_ref().map(|currency| ItemCurrencyData {
                unknown1: currency.unknown1,
            }),
            slip: self.slip.as_ref().map(|slip| ItemSlipData {
                unknown1: slip.unknown1,
                unknowns: slip.unknowns,
            }),
            monipulator: self
                .monipulator
                .as_ref()
                .map(|monipulator| ItemMonipulatorData {
                    unknown1: monipulator.unknown1,
                    unknowns: monipulator.unknowns,
                }),
        }
    }

    fn string_content(&self, index: usize, value: &str) -> Result<ItemStringContent> {
        if let Some(raw) = self.raw_strings.get(index)
            && Decoder::decode_simple(&raw.bytes)? == value
        {
            return ItemStringContent::from_bytes(&raw.bytes);
        }
        ItemStringContent::from_string(value)
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ItemInfo> {
        let mut item_bytes = walker.take_bytes(ENTRY_SIZE)?.to_vec();
        rotate_all(&mut item_bytes, 5);

        // Parse the icon
        let mut icon_walker = BufferedByteWalker::on(&item_bytes[0x280..]);
        let icon_size = icon_walker.step::<u32>()?;
        let icon_bytes = icon_walker.take_bytes(icon_size as usize)?.to_vec();

        let unterminated_icon_padding =
            icon_size == 0 && item_bytes[0x284..].iter().all(|padding| *padding == 0);
        if unterminated_icon_padding {
            icon_walker.expect_n_msg::<u8>(0, icon_walker.remaining(), "Padding after icon")?;
        } else {
            icon_walker.expect_n_msg::<u8>(0, icon_walker.remaining() - 1, "Padding after icon")?;
            icon_walker.expect_msg::<u8>(0xFF, "End of icon bytes")?;
        }

        // Parse the data
        let mut data_walker: BufferedByteWalker<&[u8]> =
            BufferedByteWalker::on(&item_bytes[..0x280]);

        let mut item_info = ItemInfo {
            icon_bytes,
            unterminated_icon_padding,
            ..Default::default()
        };

        item_info.id = data_walker.step::<u32>()?;
        let item_category = ItemCategory::from_id(item_info.id);

        // TODO: Monipulators seems to have a totally different structure than other items,
        //       since the values it gets for the following are non-sensical.

        item_info.flags = ItemFlag::from_bits_retain(data_walker.step::<u16>()?);
        item_info.stack_size = data_walker.step::<u16>()?;
        item_info.item_type = ItemType::from(data_walker.step::<u16>()?);
        item_info.resource_id = data_walker.step::<u16>()?;
        item_info.valid_targets = ValidTargets::from_bits_retain(data_walker.step::<u16>()?);

        let layout = Self::detect_layout(&item_bytes[..0x280], &item_category)?;

        if item_category == ItemCategory::Armor || item_category == ItemCategory::Weapon {
            let level = data_walker.step::<u16>()?;
            let slots = EquipmentSlot::from_bits_retain(data_walker.step::<u16>()?);
            let races = Race::from_bits_retain(data_walker.step::<u16>()?);
            let jobs = JobFlag::from_bits_retain(data_walker.step::<u32>()?);
            let superior_level = if layout == ItemLayout::Modern {
                Some(data_walker.step::<u16>()?)
            } else {
                None
            };
            let shield_size = if layout == ItemLayout::Modern {
                Some(data_walker.step::<u16>()?)
            } else {
                None
            };

            if item_category == ItemCategory::Weapon {
                item_info.weapon = Some(WeaponData {
                    damage: data_walker.step::<u16>()?,
                    delay: data_walker.step::<u16>()?,
                    dps: data_walker.step::<u16>()?,
                    skill_type: SkillType::try_from(data_walker.step::<u8>()?)?,
                    jug_size: data_walker.step::<u8>()?,
                    unknown1: if layout == ItemLayout::Modern {
                        Some(data_walker.step::<u32>()?)
                    } else {
                        None
                    },
                });
            }

            let max_charges = data_walker.step::<u8>()?;
            let casting_time = data_walker.step::<u8>()?;
            let use_delay = data_walker.step::<u16>()?;
            let reuse_delay = data_walker.step::<u32>()?;
            let unknown1 = data_walker.step::<u16>()?;
            let ilevel = data_walker.step::<u8>()?;
            let unknown2 = data_walker.step::<u8>()?;
            let unknown3 = data_walker.step::<u32>()?;

            item_info.equipment = Some(EquipmentData {
                level,
                slots,
                races,
                jobs,
                superior_level,
                shield_size,
                max_charges,
                casting_time,
                use_delay,
                reuse_delay,
                unknown1,
                ilevel,
                unknown2,
                unknown3,
            });
        } else if item_category == ItemCategory::PuppetItem {
            item_info.puppet = Some(PuppetItemData {
                slot: PuppetSlot::try_from(data_walker.step::<u16>()?)?,
                element_charge: ElementValues::from(data_walker.step::<u32>()?),
                unknown1: data_walker.step::<u32>()?,
            });
        } else if item_category == ItemCategory::Instinct {
            item_info.instinct = Some(InstinctData {
                unknown1: data_walker.step::<u32>()?,
                unknown2: data_walker.step::<u32>()?,
                unknown3: data_walker.step::<u16>()?,
                instinct_cost: data_walker.step::<u16>()?,
                unknown4: data_walker.step::<u16>()?,
                unknown5: data_walker.step::<u32>()?,
                unknown6: data_walker.step::<u32>()?,
                unknown7: data_walker.step::<u32>()?,
            });
        } else if item_category == ItemCategory::Item {
            item_info.furnishing = Some(FurnishingData {
                element: Element::try_from(data_walker.step::<u16>()?)?,
                storage_slots: data_walker.step::<u32>()?,
                unknown3: if layout == ItemLayout::Modern {
                    Some(data_walker.step::<u32>()?)
                } else {
                    None
                },
            });
        } else if item_category == ItemCategory::UsableItem {
            item_info.usable_item = Some(UsableItemData {
                activation_time: data_walker.step::<u16>()?,
                unknown1: data_walker.step::<u32>()?,
                unknown2: if layout == ItemLayout::Modern {
                    Some(data_walker.step::<u32>()?)
                } else {
                    None
                },
                unknown3: if layout == ItemLayout::Modern {
                    Some(data_walker.step::<u32>()?)
                } else {
                    None
                },
            });
        } else if item_category == ItemCategory::Currency {
            item_info.currency = Some(CurrencyData {
                unknown1: data_walker.step::<u16>()?,
            });
        } else if item_category == ItemCategory::Slip {
            item_info.slip = Some(SlipData {
                unknown1: data_walker.step::<u16>()?,
                unknowns: core::array::from_fn(|_| data_walker.step::<u32>().unwrap_or_default()),
            });
        } else if item_category == ItemCategory::Monipulator {
            item_info.monipulator = Some(MonipulatorData {
                unknown1: data_walker.step::<u16>()?,
                unknowns: core::array::from_fn(|_| data_walker.step::<u32>().unwrap_or_default()),
            });
        }

        let parsed_strings = Self::parse_strings(&mut data_walker)?;
        item_info.strings = parsed_strings.strings;
        item_info.raw_strings = parsed_strings.raw_strings;

        data_walker.expect_n_msg::<u32>(
            0,
            data_walker.remaining() / 4,
            "Zero padding at end of data",
        )?;

        Ok(item_info)
    }

    fn detect_layout(data: &[u8], item_category: &ItemCategory) -> Result<ItemLayout> {
        let offsets = match item_category {
            ItemCategory::Item => Some((0x14, 0x18)),
            ItemCategory::UsableItem => Some((0x14, 0x1C)),
            ItemCategory::Armor => Some((0x28, 0x2C)),
            ItemCategory::Weapon => Some((0x30, 0x38)),
            _ => None,
        };
        let Some((legacy_offset, modern_offset)) = offsets else {
            return Ok(ItemLayout::Modern);
        };

        let legacy_valid = Self::check_strings_at(data, legacy_offset).is_ok();
        let modern_valid = Self::check_strings_at(data, modern_offset).is_ok();

        match (legacy_valid, modern_valid) {
            (true, false) => Ok(ItemLayout::Legacy),
            (false, true) => Ok(ItemLayout::Modern),
            // Both candidates only consume the full tail when the compact
            // count and added modern fields are zero, yielding identical bytes.
            (true, true) => Ok(ItemLayout::Modern),
            (false, false) => Err(anyhow!("Unsupported item data layout")),
        }
    }

    fn check_strings_at(data: &[u8], offset: usize) -> Result<()> {
        let mut walker = BufferedByteWalker::on(data);
        walker.goto_usize(offset);
        Self::parse_strings(&mut walker)?;
        walker.expect_n_msg::<u32>(0, walker.remaining() / 4, "Zero padding at end of data")?;
        Ok(())
    }

    fn parse_strings<T: ByteWalker>(data_walker: &mut T) -> Result<ParsedItemStrings> {
        let content_start = data_walker.offset();
        let content_count = data_walker.step::<u32>()?;
        if content_count > 9 {
            return Err(anyhow!(
                "Unsupported strings content of length: {}",
                content_count
            ));
        }

        let mut metas = Vec::with_capacity(content_count as usize);
        for _ in 0..content_count {
            metas.push((data_walker.step::<u32>()?, data_walker.step::<u32>()?));
        }

        let mut raw_pairs = Vec::new();
        let strings = match content_count {
            0 => None,
            1 => {
                // Just one string name
                Self::expect_content_meta(data_walker, content_start, &metas, 0, 0)?;
                let (name, raw) = Self::read_string(data_walker)?;
                raw_pairs.push((name.clone(), raw));
                Some(ItemStrings::Name { name })
            }
            2 => {
                // Japanese
                Self::expect_content_meta(data_walker, content_start, &metas, 0, 0)?;
                let (name, raw_name) = Self::read_string(data_walker)?;
                Self::expect_content_meta(data_walker, content_start, &metas, 1, 0)?;
                let (description, raw_description) = Self::read_string(data_walker)?;
                raw_pairs.push((name.clone(), raw_name));
                raw_pairs.push((description.clone(), raw_description));
                Some(ItemStrings::Japanese { name, description })
            }
            5 => {
                // English
                Self::expect_content_meta(data_walker, content_start, &metas, 0, 0)?;
                let (name, raw_name) = Self::read_string(data_walker)?;
                raw_pairs.push((name.clone(), raw_name));
                Self::expect_content_meta(data_walker, content_start, &metas, 1, 1)?;
                let article_type = EnglishArticle::try_from(data_walker.step::<u32>()?)?;
                Self::expect_content_meta(data_walker, content_start, &metas, 2, 0)?;
                let (singular_name, raw_singular_name) = Self::read_string(data_walker)?;
                Self::expect_content_meta(data_walker, content_start, &metas, 3, 0)?;
                let (plural_name, raw_plural_name) = Self::read_string(data_walker)?;
                Self::expect_content_meta(data_walker, content_start, &metas, 4, 0)?;
                let (description, raw_description) = Self::read_string(data_walker)?;
                raw_pairs.push((singular_name.clone(), raw_singular_name));
                raw_pairs.push((plural_name.clone(), raw_plural_name));
                raw_pairs.push((description.clone(), raw_description));
                Some(ItemStrings::English {
                    name,
                    article_type,
                    singular_name,
                    plural_name,
                    description,
                })
            }
            count => {
                return Err(anyhow!("Unsupported string count: {}", count));
            }
        };

        let raw_strings = if raw_pairs.iter().all(|(string, raw)| {
            Encoder::encode_simple(string).is_ok_and(|encoded| encoded == raw.bytes)
        }) {
            Vec::new()
        } else {
            raw_pairs.into_iter().map(|(_, raw)| raw).collect()
        };

        Ok(ParsedItemStrings {
            strings,
            raw_strings,
        })
    }

    fn expect_content_meta<T: ByteWalker>(
        walker: &T,
        content_start: usize,
        metas: &[(u32, u32)],
        index: usize,
        content_type: u32,
    ) -> Result<()> {
        let expected_offset = walker.offset().saturating_sub(content_start) as u32;
        let Some((offset, actual_type)) = metas.get(index) else {
            return Err(anyhow!("Missing item string metadata"));
        };
        if *offset != expected_offset || *actual_type != content_type {
            return Err(anyhow!("Invalid item string metadata"));
        }
        Ok(())
    }

    fn read_string<T: ByteWalker>(walker: &mut T) -> Result<(String, EncodedStringBytes)> {
        walker.expect_msg::<u32>(1, "Expected 1 at start of string.")?;
        walker.expect_n_msg::<u32>(0, 6, "Expected 0 padding before string.")?;

        let text_bytes = walker.step_until(0)?.to_vec();
        let string = Decoder::decode_simple(&text_bytes)?;

        let alignment_padding = get_padding(text_bytes.len() + 1);
        walker.expect_msg::<u8>(0, "End of string")?;
        walker.expect_n_msg::<u8>(0, alignment_padding, "Expected 0 padding after string.")?;

        Ok((string, EncodedStringBytes { bytes: text_bytes }))
    }

    pub fn write<T: WritingByteWalker>(&self, outer_walker: &mut T) -> Result<()> {
        let mut walker = VecByteWalker::with_size(ENTRY_SIZE);

        walker.write(self.id);
        walker.write(self.flags.bits());

        // Write item data
        walker.write(self.stack_size);
        walker.write::<u16>(self.item_type.into());
        walker.write(self.resource_id);
        walker.write(self.valid_targets.bits());

        if let Some(equipment) = &self.equipment {
            let modern_equipment = match (equipment.superior_level, equipment.shield_size) {
                (Some(_), Some(_)) => true,
                (None, None) => false,
                _ => return Err(anyhow!("Incomplete equipment layout fields")),
            };
            if self
                .weapon
                .as_ref()
                .is_some_and(|weapon| weapon.unknown1.is_some() != modern_equipment)
            {
                return Err(anyhow!("Mixed equipment and weapon layouts"));
            }

            walker.write(equipment.level);
            walker.write(equipment.slots.bits());
            walker.write(equipment.races.bits());
            walker.write(equipment.jobs.bits());
            if let (Some(superior_level), Some(shield_size)) =
                (equipment.superior_level, equipment.shield_size)
            {
                walker.write(superior_level);
                walker.write(shield_size);
            }

            if let Some(weapon) = &self.weapon {
                walker.write(weapon.damage);
                walker.write(weapon.delay);
                walker.write(weapon.dps);
                walker.write::<u8>(weapon.skill_type.into());
                walker.write(weapon.jug_size);
                if let Some(unknown1) = weapon.unknown1 {
                    walker.write(unknown1);
                }
            }

            walker.write(equipment.max_charges);
            walker.write(equipment.casting_time);
            walker.write(equipment.use_delay);
            walker.write(equipment.reuse_delay);
            walker.write(equipment.unknown1);
            walker.write(equipment.ilevel);
            walker.write(equipment.unknown2);
            walker.write(equipment.unknown3);
        } else if let Some(puppet) = &self.puppet {
            walker.write::<u16>(puppet.slot.into());
            walker.write::<u32>(puppet.element_charge.into());
            walker.write(puppet.unknown1);
        } else if let Some(instinct) = &self.instinct {
            walker.write(instinct.unknown1);
            walker.write(instinct.unknown2);
            walker.write(instinct.unknown3);
            walker.write(instinct.instinct_cost);
            walker.write(instinct.unknown4);
            walker.write(instinct.unknown5);
            walker.write(instinct.unknown6);
            walker.write(instinct.unknown7);
        } else if let Some(furnishing) = &self.furnishing {
            walker.write::<u16>(furnishing.element.into());
            walker.write(furnishing.storage_slots);
            if let Some(unknown3) = furnishing.unknown3 {
                walker.write(unknown3);
            }
        } else if let Some(usable_item) = &self.usable_item {
            if usable_item.unknown2.is_some() != usable_item.unknown3.is_some() {
                return Err(anyhow!("Incomplete usable-item layout fields"));
            }
            walker.write(usable_item.activation_time);
            walker.write(usable_item.unknown1);
            if let (Some(unknown2), Some(unknown3)) = (usable_item.unknown2, usable_item.unknown3) {
                walker.write(unknown2);
                walker.write(unknown3);
            }
        } else if let Some(currency) = &self.currency {
            walker.write(currency.unknown1);
        } else if let Some(slip) = &self.slip {
            walker.write(slip.unknown1);
            for unknown in slip.unknowns {
                walker.write(unknown);
            }
        } else if let Some(monipulator) = &self.monipulator {
            walker.write(monipulator.unknown1);
            for unknown in monipulator.unknowns {
                walker.write(unknown);
            }
        }

        // Write strings
        let mut string_content = vec![];

        match &self.strings {
            Some(ItemStrings::Name { name }) => {
                string_content.push(self.string_content(0, name)?);
            }
            Some(ItemStrings::Japanese { name, description }) => {
                string_content.push(self.string_content(0, name)?);
                string_content.push(self.string_content(1, description)?);
            }
            Some(ItemStrings::English {
                name,
                article_type,
                singular_name,
                plural_name,
                description,
            }) => {
                string_content.push(self.string_content(0, name)?);
                string_content.push(ItemStringContent::from_article(*article_type));
                string_content.push(self.string_content(1, singular_name)?);
                string_content.push(self.string_content(2, plural_name)?);
                string_content.push(self.string_content(3, description)?);
            }
            None => {}
        }

        // Write metas
        walker.write::<u32>(string_content.len() as u32);

        let mut current_offset: u32 = string_content.len() as u32 * 8 + 4;
        for content in &string_content {
            match content {
                ItemStringContent::Number(_) => {
                    walker.write::<u32>(current_offset);
                    walker.write::<u32>(1);

                    current_offset += 4;
                }
                ItemStringContent::StringBytes(string_bytes) => {
                    walker.write::<u32>(current_offset);
                    walker.write::<u32>(0);

                    current_offset += string_bytes.len() as u32;
                }
            }
        }

        // Write string content
        for content in &string_content {
            match content {
                ItemStringContent::Number(number) => {
                    walker.write(*number);
                }
                ItemStringContent::StringBytes(string_bytes) => {
                    walker.write_bytes(string_bytes);
                }
            }
        }

        // Write icon bytes
        walker.goto(0x280);
        walker.write(self.icon_bytes.len() as u32);
        walker.write_bytes(&self.icon_bytes);
        if self.unterminated_icon_padding {
            if !self.icon_bytes.is_empty() {
                return Err(anyhow!(
                    "Unterminated item icon padding requires an empty icon"
                ));
            }
        } else {
            walker.write_at::<u8>(ENTRY_SIZE - 1, 0xFF);
        }

        rotate_all(walker.as_mut_slice(), 3);
        outer_walker.write_bytes(walker.as_slice());

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ItemInfoTable {
    items: Vec<ItemInfo>,
}

const ENTRY_SIZE: usize = 0xC00;

impl ItemInfoTable {
    pub fn neutral_entries(&self) -> Vec<ItemEntry> {
        self.items
            .iter()
            .map(|item| ItemEntry {
                data: item.data(),
                text: item.text(),
            })
            .collect()
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!(
                "Length does not match an item info DAT: {}",
                walker.len()
            ));
        }

        let entry_count = walker.len() / ENTRY_SIZE;
        let mut items = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            items.push(ItemInfo::parse(walker)?);
        }

        Ok(ItemInfoTable { items })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.set_size(self.items.len() * ENTRY_SIZE);

        for item in &self.items {
            item.write(walker)?;
        }

        Ok(())
    }
}

impl DatFormat for ItemInfoTable {
    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        ItemInfoTable::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        if walker.len() % ENTRY_SIZE != 0 {
            return Err(anyhow!("Length does not match an item info DAT."));
        }

        // Parse one item info to check.
        ItemInfo::parse(walker)?;

        Ok(())
    }

    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use encoding::decoder::Decoder;

    use crate::{
        dat_format::DatFormat,
        enums::{Element, EnglishArticle, ItemType, SkillType},
        flags::{ItemFlag, JobFlag, Race, ValidTargets},
        utils::rotate_all,
    };

    use super::{
        EncodedStringBytes, EquipmentData, FurnishingData, ItemInfo, ItemInfoTable, ItemStrings,
        UsableItemData, WeaponData,
    };

    fn test_strings() -> Option<ItemStrings> {
        Some(ItemStrings::Name {
            name: "Test item".to_string(),
        })
    }

    fn legacy_equipment() -> EquipmentData {
        EquipmentData {
            level: 1,
            slots: Default::default(),
            races: Default::default(),
            jobs: Default::default(),
            superior_level: None,
            shield_size: None,
            max_charges: 0,
            casting_time: 0,
            use_delay: 0,
            reuse_delay: 0,
            unknown1: 0,
            ilevel: 0,
            unknown2: 0,
            unknown3: 0,
        }
    }

    #[test]
    fn legacy_item_layouts_round_trip() {
        let table = ItemInfoTable {
            items: vec![
                ItemInfo {
                    id: 1,
                    item_type: ItemType::Item,
                    strings: test_strings(),
                    furnishing: Some(FurnishingData {
                        element: Element::Undecided,
                        storage_slots: 0,
                        unknown3: None,
                    }),
                    ..Default::default()
                },
                ItemInfo {
                    id: 0x1000,
                    item_type: ItemType::UsableItem,
                    strings: test_strings(),
                    usable_item: Some(UsableItemData {
                        activation_time: 0,
                        unknown1: 0,
                        unknown2: None,
                        unknown3: None,
                    }),
                    ..Default::default()
                },
                ItemInfo {
                    id: 0x2800,
                    item_type: ItemType::Armor,
                    strings: test_strings(),
                    equipment: Some(legacy_equipment()),
                    ..Default::default()
                },
                ItemInfo {
                    id: 0x4000,
                    item_type: ItemType::Weapon,
                    strings: test_strings(),
                    equipment: Some(legacy_equipment()),
                    weapon: Some(WeaponData {
                        damage: 1,
                        delay: 1,
                        dps: 1,
                        skill_type: SkillType::Sword,
                        jug_size: 0,
                        unknown1: None,
                    }),
                    ..Default::default()
                },
            ],
        };

        let bytes = table.to_bytes().unwrap();
        assert_eq!(ItemInfoTable::from_bytes_checked(&bytes).unwrap(), table);
    }

    #[test]
    fn neutral_entries_are_numeric_and_omit_binary_fields() {
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                id: 0x4000,
                strings: Some(ItemStrings::English {
                    name: "Test sword".to_string(),
                    article_type: EnglishArticle::A,
                    singular_name: "test sword".to_string(),
                    plural_name: "test swords".to_string(),
                    description: "A synthetic weapon.".to_string(),
                }),
                raw_strings: vec![EncodedStringBytes {
                    bytes: vec![1, 2, 3],
                }],
                flags: ItemFlag::Rare | ItemFlag::NoAuction,
                stack_size: 1,
                item_type: ItemType::Weapon,
                resource_id: 7,
                valid_targets: ValidTargets::SelfTarget,
                equipment: Some(legacy_equipment()),
                weapon: Some(WeaponData {
                    damage: 12,
                    delay: 240,
                    dps: 3,
                    skill_type: SkillType::Sword,
                    jug_size: 0,
                    unknown1: None,
                }),
                icon_bytes: vec![4, 5, 6],
                ..Default::default()
            }],
        };

        let entries = table.neutral_entries();
        let entry = &entries[0];
        assert_eq!(entry.data.id, 0x4000);
        assert_eq!(entry.data.flags_bits, 0x8040);
        assert_eq!(entry.data.item_type_code, 4);
        assert_eq!(entry.data.valid_targets_bits, 1);
        assert_eq!(entry.data.weapon.as_ref().unwrap().skill_type_code, 3);
        assert_eq!(entry.text.as_ref().unwrap().article_type_code, Some(0));

        let yaml = serde_yaml::to_string(&entries).unwrap();
        assert!(!yaml.contains("icon"));
        assert!(!yaml.contains("raw_strings"));
        assert!(!yaml.contains("unterminated"));
    }

    #[test]
    fn unknown_equipment_flag_bits_survive_round_trip_and_export() {
        let mut equipment = legacy_equipment();
        equipment.races = Race::from_bits_retain(1);
        equipment.jobs = JobFlag::from_bits_retain(1);
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                id: 0x2800,
                item_type: ItemType::Armor,
                strings: test_strings(),
                equipment: Some(equipment),
                ..Default::default()
            }],
        };

        let bytes = table.to_bytes().unwrap();
        let parsed = ItemInfoTable::from_bytes_checked(&bytes).unwrap();
        let exported = parsed.neutral_entries();

        assert_eq!(exported[0].data.equipment.as_ref().unwrap().races_bits, 1);
        assert_eq!(exported[0].data.equipment.as_ref().unwrap().jobs_bits, 1);
    }

    #[test]
    fn empty_item_record_round_trips_without_icon_terminator() {
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                unterminated_icon_padding: true,
                ..Default::default()
            }],
        };

        let bytes = table.to_bytes().unwrap();
        ItemInfoTable::from_bytes_checked(&bytes).unwrap();
    }

    #[test]
    fn zero_string_compact_item_round_trips() {
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                id: 1,
                item_type: ItemType::Item,
                furnishing: Some(FurnishingData {
                    element: Element::Fire,
                    storage_slots: 1,
                    unknown3: None,
                }),
                ..Default::default()
            }],
        };

        let bytes = table.to_bytes().unwrap();
        ItemInfoTable::from_bytes_checked(&bytes).unwrap();
    }

    #[test]
    fn noncanonical_item_string_bytes_survive_yaml() {
        let raw = vec![0x81, 0x68];
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                id: 1,
                item_type: ItemType::Item,
                strings: Some(ItemStrings::Name {
                    name: Decoder::decode_simple(&raw).unwrap(),
                }),
                raw_strings: vec![EncodedStringBytes { bytes: raw }],
                furnishing: Some(FurnishingData {
                    element: Element::Undecided,
                    storage_slots: 0,
                    unknown3: None,
                }),
                ..Default::default()
            }],
        };

        let bytes = table.to_bytes().unwrap();
        let mut parsed = ItemInfoTable::from_bytes(&bytes).unwrap();
        let yaml = serde_yaml::to_string(&parsed).unwrap();
        let reread: ItemInfoTable = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(reread.to_bytes().unwrap(), bytes);

        parsed.items[0].strings = Some(ItemStrings::Name {
            name: "Edited item".to_string(),
        });
        let edited_bytes = parsed.to_bytes().unwrap();
        assert_ne!(edited_bytes, bytes);
        assert!(ItemInfoTable::from_bytes_checked(&edited_bytes).is_ok());
    }

    #[test]
    fn invalid_item_string_metadata_is_rejected() {
        let table = ItemInfoTable {
            items: vec![ItemInfo {
                id: 1,
                item_type: ItemType::Item,
                strings: test_strings(),
                furnishing: Some(FurnishingData {
                    element: Element::Undecided,
                    storage_slots: 0,
                    unknown3: None,
                }),
                ..Default::default()
            }],
        };

        let mut bytes = table.to_bytes().unwrap();
        rotate_all(&mut bytes, 5);
        bytes[0x18..0x1C].copy_from_slice(&0u32.to_le_bytes());
        rotate_all(&mut bytes, 3);
        assert!(ItemInfoTable::from_bytes(&bytes).is_err());
    }

    #[test]
    pub fn weapons() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/weapons.DAT");

        ItemInfoTable::check_path(&dat_path).unwrap();
        let res = ItemInfoTable::from_path_checked_yaml(&dat_path).unwrap();

        if let ItemStrings::English {
            name,
            article_type,
            singular_name,
            plural_name,
            description,
        } = res.items[4329].strings.as_ref().unwrap()
        {
            assert_eq!(name, "Excalipoor");
            assert_eq!(article_type, &EnglishArticle::An);
            assert_eq!(singular_name, "Excalipoor");
            assert_eq!(plural_name, "Excalipoors");
            assert_eq!(description, "DMG:1 Delay:240");
        } else {
            panic!("Expected english strings")
        }
    }

    #[test]
    pub fn armor2() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/armor2.DAT");

        ItemInfoTable::check_path(&dat_path).unwrap();
        let res = ItemInfoTable::from_path_checked_yaml(&dat_path).unwrap();

        if let ItemStrings::English {
            name,
            article_type,
            singular_name,
            plural_name,
            description,
        } = res.items[3827].strings.as_ref().unwrap()
        {
            assert_eq!(name, "Voodoo Mail");
            assert_eq!(article_type, &EnglishArticle::SuitsOf);
            assert_eq!(singular_name, "voodoo mail");
            assert_eq!(plural_name, "suits of voodoo mail");
            assert_eq!(
                description,
                "The envious aura that looms over\nthis mail seems to invite utter\nruin to descend upon its bearer."
            );
        } else {
            panic!("Expected english strings")
        }
    }

    #[test]
    pub fn armor_jp() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/armor_jp.DAT");

        ItemInfoTable::check_path(&dat_path).unwrap();
        let res = ItemInfoTable::from_path_checked_yaml(&dat_path).unwrap();

        if let ItemStrings::Japanese { name, description } =
            res.items[2221].strings.as_ref().unwrap()
        {
            assert_eq!(name, "スコピオヘルム+1");
            assert_eq!(
                description,
                "防23 耐火+8 レジストパライズ効果アップ\n麻痺:リフレシュ"
            );
        } else {
            panic!("Expected japanese strings")
        }
    }
}
