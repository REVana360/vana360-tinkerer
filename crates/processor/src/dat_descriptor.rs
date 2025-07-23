use anyhow::{anyhow, Result};
use std::{path::PathBuf, sync::Arc};

use dats::{
    base::{Dat, ZoneId},
    context::DatContext,
    dat_format::DatFormat,
    id_mapping::DatIdMapping,
};
use serde::{Deserialize, Serialize};

use crate::converters::{DatToYamlConverter, YamlToDatConverter};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, specta::Type, Serialize, Deserialize,
)]
#[serde(tag = "type", content = "index")]
pub enum DatDescriptor {
    DataMenu,
    QuestsMissionsKeyItems,

    // String tables
    AbilityNames,
    AbilityDescriptions,
    AreaNames,
    AreaNamesShort,
    AreaNamesAlt,
    Augments,
    BlueMagic,
    CallMount,
    CharacterSelect,
    ChatFilterTypes,
    ChocoboNames,
    CommandUsage,
    DayNames,
    Directions,
    EinherjarChambers,
    Emotes,
    EquipmentLocations,
    EquipmentLocationsAlt,
    ErrorMessages,
    IngameMessages1,
    IngameMessages2,
    JobNames,
    JobNamesShort,
    JobPointBonuses,
    JobPointGifts,
    KeyItems,
    MenuItemsDescription,
    MenuItemsText,
    Merits,
    MissionsAcp,
    MissionsAmke,
    MissionsAsa,
    MissionsAssault,
    MissionsBastok,
    MissionsCampaign,
    MissionsCop,
    MissionsRov,
    MissionsSandoria,
    MissionsSoa,
    MissionsToau,
    MissionsWindurst,
    MissionsWotg,
    MissionsZilart,
    MoblinMazeMongers,
    Modifiers,
    MonsterFamilies,
    MoonPhases,
    PankrationNames,
    PolMessages,
    QuestsAbyssea,
    QuestsBastok,
    QuestsCoalition,
    QuestsJeuno,
    QuestsOther,
    QuestsOutlands,
    QuestsSandoria,
    QuestsSoa,
    QuestsToau,
    QuestsWindurst,
    QuestsWotg,
    RaceNames,
    RegionNames,
    ServerNames,
    SpellNames,
    SpellDescriptions,
    StatusInfo,
    StatusNames,
    TimeAndPronouns,
    Titles,
    TrustMessages,
    Misc1,
    Misc2,
    WeatherTypes,

    // Item data
    Armor,
    Armor2,
    Currency,
    GeneralItems,
    GeneralItems2,
    PuppetItems,
    UsableItems,
    Weapons,
    VouchersAndSlips,
    Monipulator,
    Instincts,

    // Global dialog
    MonsterSkillNames,
    StatusNamesDialog,
    EmoteMessages,
    SystemMessages1,
    SystemMessages2,
    SystemMessages3,
    SystemMessages4,
    UnityDialogs,

    // Dats by zone
    ZoneData(ZoneId),
    EntityNames(ZoneId),
    Dialog(ZoneId),
    Dialog2(ZoneId),
    Events(ZoneId),
}

pub trait DatUsage {
    fn use_dat<T: DatFormat + Serialize + for<'a> serde::Deserialize<'a>>(
        self,
        dat: Dat<T>,
    ) -> Result<PathBuf>;
}

impl DatDescriptor {
    pub fn dat_to_yaml(
        &self,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
    ) -> Result<PathBuf> {
        let data_path = raw_data_root_path.join(self.get_relative_path(&dat_context)? + ".yml");
        self.convert_with(DatToYamlConverter {
            dat_context,
            raw_data_path: data_path,
        })
    }

    pub fn yaml_to_dat(
        &self,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
        dat_root_path: PathBuf,
    ) -> Result<PathBuf> {
        let raw_data_path = raw_data_root_path.join(self.get_relative_path(&dat_context)? + ".yml");
        self.convert_with(YamlToDatConverter {
            dat_context,
            raw_data_path,
            dat_root_path,
        })
    }

    fn get_zoned_file_name(
        dat_context: &DatContext,
        dir_name: &'static str,
        zone_id: &u16,
    ) -> Result<String> {
        Ok(format!(
            "{}/{}",
            dir_name,
            dat_context
                .zone_id_to_name
                .get(&zone_id)
                .ok_or(anyhow!("No zone name found for zone ID."))?
                .file_name
        ))
    }

    fn get_relative_path(&self, dat_context: &DatContext) -> Result<String> {
        match self {
            DatDescriptor::DataMenu => Ok("data_menu".to_string()),
            DatDescriptor::QuestsMissionsKeyItems => Ok("quests_mission_keyitems".to_string()),

            DatDescriptor::AbilityNames => Ok("ability_names".to_string()),
            DatDescriptor::AbilityDescriptions => Ok("ability_descriptions".to_string()),
            DatDescriptor::AreaNames => Ok("area_names".to_string()),
            DatDescriptor::AreaNamesShort => Ok("area_names_short".to_string()),
            DatDescriptor::AreaNamesAlt => Ok("area_names_alt".to_string()),
            DatDescriptor::Augments => Ok("augments".to_string()),
            DatDescriptor::BlueMagic => Ok("blue_magic".to_string()),
            DatDescriptor::CallMount => Ok("call_mount".to_string()),
            DatDescriptor::CharacterSelect => Ok("character_select".to_string()),
            DatDescriptor::ChatFilterTypes => Ok("chat_filter_types".to_string()),
            DatDescriptor::ChocoboNames => Ok("chocobo_names".to_string()),
            DatDescriptor::CommandUsage => Ok("command_usage".to_string()),
            DatDescriptor::Emotes => Ok("emotes".to_string()),
            DatDescriptor::EinherjarChambers => Ok("einherjar_chambers".to_string()),
            DatDescriptor::DayNames => Ok("day_names".to_string()),
            DatDescriptor::Directions => Ok("directions".to_string()),
            DatDescriptor::EquipmentLocations => Ok("equipment_locations".to_string()),
            DatDescriptor::EquipmentLocationsAlt => Ok("equipment_locations_alt".to_string()),
            DatDescriptor::ErrorMessages => Ok("error_messages".to_string()),
            DatDescriptor::IngameMessages1 => Ok("ingame_messages1".to_string()),
            DatDescriptor::IngameMessages2 => Ok("ingame_messages2".to_string()),
            DatDescriptor::JobNames => Ok("job_names".to_string()),
            DatDescriptor::JobNamesShort => Ok("job_names_short".to_string()),
            DatDescriptor::JobPointBonuses => Ok("job_point_bonuses".to_string()),
            DatDescriptor::JobPointGifts => Ok("job_point_gifts".to_string()),
            DatDescriptor::KeyItems => Ok("key_items".to_string()),
            DatDescriptor::MenuItemsDescription => Ok("menu_items_description".to_string()),
            DatDescriptor::MenuItemsText => Ok("menu_items_text".to_string()),
            DatDescriptor::Merits => Ok("merits".to_string()),
            DatDescriptor::Modifiers => Ok("modifiers".to_string()),
            DatDescriptor::MissionsAcp => Ok("missions_acp".to_string()),
            DatDescriptor::MissionsAmke => Ok("missions_amke".to_string()),
            DatDescriptor::MissionsAsa => Ok("missions_asa".to_string()),
            DatDescriptor::MissionsAssault => Ok("missions_assault".to_string()),
            DatDescriptor::MissionsBastok => Ok("missions_bastok".to_string()),
            DatDescriptor::MissionsCampaign => Ok("missions_campaign".to_string()),
            DatDescriptor::MissionsCop => Ok("missions_cop".to_string()),
            DatDescriptor::MissionsRov => Ok("missions_rov".to_string()),
            DatDescriptor::MissionsSandoria => Ok("missions_sandoria".to_string()),
            DatDescriptor::MissionsSoa => Ok("missions_soa".to_string()),
            DatDescriptor::MissionsToau => Ok("missions_toau".to_string()),
            DatDescriptor::MissionsWindurst => Ok("missions_windurst".to_string()),
            DatDescriptor::MissionsWotg => Ok("missions_wotg".to_string()),
            DatDescriptor::MissionsZilart => Ok("missions_zilart".to_string()),
            DatDescriptor::MoblinMazeMongers => Ok("moblin_maze_mongers".to_string()),
            DatDescriptor::MonsterFamilies => Ok("monster_families".to_string()),
            DatDescriptor::MoonPhases => Ok("moon_phases".to_string()),
            DatDescriptor::PankrationNames => Ok("pankration_names".to_string()),
            DatDescriptor::PolMessages => Ok("pol_messages".to_string()),
            DatDescriptor::QuestsAbyssea => Ok("quests_abyssea".to_string()),
            DatDescriptor::QuestsBastok => Ok("quests_bastok".to_string()),
            DatDescriptor::QuestsCoalition => Ok("quests_coalition".to_string()),
            DatDescriptor::QuestsJeuno => Ok("quests_jeuno".to_string()),
            DatDescriptor::QuestsOther => Ok("quests_other".to_string()),
            DatDescriptor::QuestsOutlands => Ok("quests_outlands".to_string()),
            DatDescriptor::QuestsSandoria => Ok("quests_sandoria".to_string()),
            DatDescriptor::QuestsSoa => Ok("quests_soa".to_string()),
            DatDescriptor::QuestsToau => Ok("quests_toau".to_string()),
            DatDescriptor::QuestsWindurst => Ok("quests_windurst".to_string()),
            DatDescriptor::QuestsWotg => Ok("quests_wotg".to_string()),
            DatDescriptor::RaceNames => Ok("race_names".to_string()),
            DatDescriptor::RegionNames => Ok("region_names".to_string()),
            DatDescriptor::ServerNames => Ok("server_names".to_string()),
            DatDescriptor::SpellNames => Ok("spell_names".to_string()),
            DatDescriptor::SpellDescriptions => Ok("spell_descriptions".to_string()),
            DatDescriptor::StatusInfo => Ok("status_info".to_string()),
            DatDescriptor::StatusNames => Ok("status_names".to_string()),
            DatDescriptor::TimeAndPronouns => Ok("time_and_pronouns".to_string()),
            DatDescriptor::Titles => Ok("titles".to_string()),
            DatDescriptor::TrustMessages => Ok("trust_messages".to_string()),
            DatDescriptor::Misc1 => Ok("misc1".to_string()),
            DatDescriptor::Misc2 => Ok("misc2".to_string()),
            DatDescriptor::WeatherTypes => Ok("weather_types".to_string()),

            DatDescriptor::Armor => Ok("items/armor".to_string()),
            DatDescriptor::Armor2 => Ok("items/armor2".to_string()),
            DatDescriptor::Currency => Ok("items/currency".to_string()),
            DatDescriptor::GeneralItems => Ok("items/general_items".to_string()),
            DatDescriptor::GeneralItems2 => Ok("items/general_items2".to_string()),
            DatDescriptor::PuppetItems => Ok("items/puppet_items".to_string()),
            DatDescriptor::UsableItems => Ok("items/usable_items".to_string()),
            DatDescriptor::Weapons => Ok("items/weapons".to_string()),
            DatDescriptor::VouchersAndSlips => Ok("items/vouchers_and_slips".to_string()),
            DatDescriptor::Monipulator => Ok("items/monipulator".to_string()),
            DatDescriptor::Instincts => Ok("items/instincts".to_string()),

            DatDescriptor::MonsterSkillNames => Ok("global_dialog/monster_skill_names".to_string()),
            DatDescriptor::StatusNamesDialog => Ok("global_dialog/status_names_dialog".to_string()),
            DatDescriptor::EmoteMessages => Ok("global_dialog/emote_messages".to_string()),
            DatDescriptor::SystemMessages1 => Ok("global_dialog/system_messages1".to_string()),
            DatDescriptor::SystemMessages2 => Ok("global_dialog/system_messages2".to_string()),
            DatDescriptor::SystemMessages3 => Ok("global_dialog/system_messages3".to_string()),
            DatDescriptor::SystemMessages4 => Ok("global_dialog/system_messages4".to_string()),
            DatDescriptor::UnityDialogs => Ok("global_dialog/unity_dialogs".to_string()),

            DatDescriptor::ZoneData(zone_id) => {
                Self::get_zoned_file_name(dat_context, "zones", zone_id)
            }
            DatDescriptor::EntityNames(zone_id) => {
                Self::get_zoned_file_name(dat_context, "entity_names", zone_id)
            }
            DatDescriptor::Dialog(zone_id) => {
                Self::get_zoned_file_name(dat_context, "dialog", zone_id)
            }
            DatDescriptor::Dialog2(zone_id) => {
                Self::get_zoned_file_name(dat_context, "dialog2", zone_id)
            }
            DatDescriptor::Events(zone_id) => {
                Self::get_zoned_file_name(dat_context, "events", zone_id)
            }
        }
    }

    fn get_zone_id(zone_dir_name: &str, dat_context: &DatContext) -> Option<ZoneId> {
        dat_context.zone_name_to_id_map.get(zone_dir_name).copied()
    }

    pub fn from_path(
        path: &PathBuf,
        raw_data_dir: &PathBuf,
        dat_context: &DatContext,
    ) -> Option<Self> {
        let path = path.strip_prefix(raw_data_dir).unwrap_or(path);

        let file_name = path
            .file_name()
            .and_then(|osstr| osstr.to_str())
            .map(|s| s.trim_end_matches(".yml"))?;

        if let Some(parent) = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|osstr| osstr.to_str())
        {
            // Files in sub-directories
            return match parent {
                "zones" => Self::get_zone_id(file_name, dat_context).map(DatDescriptor::ZoneData),
                "entity_names" => {
                    Self::get_zone_id(file_name, dat_context).map(DatDescriptor::EntityNames)
                }
                "dialog" => Self::get_zone_id(file_name, dat_context).map(DatDescriptor::Dialog),
                "dialog2" => Self::get_zone_id(file_name, dat_context).map(DatDescriptor::Dialog2),
                "events" => Self::get_zone_id(file_name, dat_context).map(DatDescriptor::Events),

                "items" => match file_name {
                    "armor" => Some(DatDescriptor::Armor),
                    "armor2" => Some(DatDescriptor::Armor2),
                    "currency" => Some(DatDescriptor::Currency),
                    "general_items" => Some(DatDescriptor::GeneralItems),
                    "general_items2" => Some(DatDescriptor::GeneralItems2),
                    "puppet_items" => Some(DatDescriptor::PuppetItems),
                    "usable_items" => Some(DatDescriptor::UsableItems),
                    "weapons" => Some(DatDescriptor::Weapons),
                    "vouchers_and_slips" => Some(DatDescriptor::VouchersAndSlips),
                    "monipulator" => Some(DatDescriptor::Monipulator),
                    "instincts" => Some(DatDescriptor::Instincts),
                    _ => None,
                },
                "global_dialog" => match file_name {
                    "monster_skill_names" => Some(DatDescriptor::MonsterSkillNames),
                    "status_names_dialog" => Some(DatDescriptor::StatusNamesDialog),
                    "emote_messages" => Some(DatDescriptor::EmoteMessages),
                    "system_messages1" => Some(DatDescriptor::SystemMessages1),
                    "system_messages2" => Some(DatDescriptor::SystemMessages2),
                    "system_messages3" => Some(DatDescriptor::SystemMessages3),
                    "system_messages4" => Some(DatDescriptor::SystemMessages4),
                    "unity_dialogs" => Some(DatDescriptor::UnityDialogs),
                    _ => None,
                },
                _ => {
                    println!("Parent is: {}", parent);
                    None
                }
            };
        }

        // Files in root directory
        match file_name {
            "ability_names" => Some(DatDescriptor::AbilityNames),
            "ability_descriptions" => Some(DatDescriptor::AbilityDescriptions),
            "area_names" => Some(DatDescriptor::AreaNames),
            "area_names_short" => Some(DatDescriptor::AreaNamesShort),
            "area_names_alt" => Some(DatDescriptor::AreaNamesAlt),
            "augments" => Some(DatDescriptor::Augments),
            "blue_magic" => Some(DatDescriptor::BlueMagic),
            "call_mount" => Some(DatDescriptor::CallMount),
            "character_select" => Some(DatDescriptor::CharacterSelect),
            "chat_filter_types" => Some(DatDescriptor::ChatFilterTypes),
            "chocobo_names" => Some(DatDescriptor::ChocoboNames),
            "command_usage" => Some(DatDescriptor::CommandUsage),
            "einherjar_chambers" => Some(DatDescriptor::EinherjarChambers),
            "emotes" => Some(DatDescriptor::Emotes),
            "day_names" => Some(DatDescriptor::DayNames),
            "directions" => Some(DatDescriptor::Directions),
            "equipment_locations" => Some(DatDescriptor::EquipmentLocations),
            "equipment_locations_alt" => Some(DatDescriptor::EquipmentLocationsAlt),
            "error_messages" => Some(DatDescriptor::ErrorMessages),
            "ingame_messages1" => Some(DatDescriptor::IngameMessages1),
            "ingame_messages2" => Some(DatDescriptor::IngameMessages2),
            "job_names" => Some(DatDescriptor::JobNames),
            "job_names_short" => Some(DatDescriptor::JobNamesShort),
            "job_point_bonuses" => Some(DatDescriptor::JobPointBonuses),
            "job_point_gifts" => Some(DatDescriptor::JobPointGifts),
            "key_items" => Some(DatDescriptor::KeyItems),
            "data_menu" => Some(DatDescriptor::DataMenu),
            "quests_mission_keyitems" => Some(DatDescriptor::QuestsMissionsKeyItems),
            "menu_items_description" => Some(DatDescriptor::MenuItemsDescription),
            "menu_items_text" => Some(DatDescriptor::MenuItemsText),
            "merits" => Some(DatDescriptor::Merits),
            "missions_acp" => Some(DatDescriptor::MissionsAcp),
            "missions_amke" => Some(DatDescriptor::MissionsAmke),
            "missions_asa" => Some(DatDescriptor::MissionsAsa),
            "missions_assault" => Some(DatDescriptor::MissionsAssault),
            "missions_bastok" => Some(DatDescriptor::MissionsBastok),
            "missions_campaign" => Some(DatDescriptor::MissionsCampaign),
            "missions_cop" => Some(DatDescriptor::MissionsCop),
            "missions_rov" => Some(DatDescriptor::MissionsRov),
            "missions_sandoria" => Some(DatDescriptor::MissionsSandoria),
            "missions_soa" => Some(DatDescriptor::MissionsSoa),
            "missions_toau" => Some(DatDescriptor::MissionsToau),
            "missions_windurst" => Some(DatDescriptor::MissionsWindurst),
            "missions_wotg" => Some(DatDescriptor::MissionsWotg),
            "missions_zilart" => Some(DatDescriptor::MissionsZilart),
            "moblin_maze_mongers" => Some(DatDescriptor::MoblinMazeMongers),
            "modifiers" => Some(DatDescriptor::Modifiers),
            "monster_families" => Some(DatDescriptor::MonsterFamilies),
            "moon_phases" => Some(DatDescriptor::MoonPhases),
            "pankration_names" => Some(DatDescriptor::PankrationNames),
            "quests_abyssea" => Some(DatDescriptor::QuestsAbyssea),
            "quests_bastok" => Some(DatDescriptor::QuestsBastok),
            "quests_coalition" => Some(DatDescriptor::QuestsCoalition),
            "quests_jeuno" => Some(DatDescriptor::QuestsJeuno),
            "quests_other" => Some(DatDescriptor::QuestsOther),
            "quests_outlands" => Some(DatDescriptor::QuestsOutlands),
            "quests_sandoria" => Some(DatDescriptor::QuestsSandoria),
            "quests_soa" => Some(DatDescriptor::QuestsSoa),
            "quests_toau" => Some(DatDescriptor::QuestsToau),
            "quests_windurst" => Some(DatDescriptor::QuestsWindurst),
            "quests_wotg" => Some(DatDescriptor::QuestsWotg),
            "pol_messages" => Some(DatDescriptor::PolMessages),
            "race_names" => Some(DatDescriptor::RaceNames),
            "region_names" => Some(DatDescriptor::RegionNames),
            "server_names" => Some(DatDescriptor::ServerNames),
            "spell_names" => Some(DatDescriptor::SpellNames),
            "spell_descriptions" => Some(DatDescriptor::SpellDescriptions),
            "status_info" => Some(DatDescriptor::StatusInfo),
            "status_names" => Some(DatDescriptor::StatusNames),
            "time_and_pronouns" => Some(DatDescriptor::TimeAndPronouns),
            "titles" => Some(DatDescriptor::Titles),
            "trust_messages" => Some(DatDescriptor::TrustMessages),
            "misc1" => Some(DatDescriptor::Misc1),
            "misc2" => Some(DatDescriptor::Misc2),
            "weather_types" => Some(DatDescriptor::WeatherTypes),

            _ => None,
        }
    }

    fn convert_with<T: DatUsage>(self, converter: T) -> Result<PathBuf> {
        match self {
            DatDescriptor::DataMenu => converter.use_dat(DatIdMapping::get().data_menu.clone()),
            DatDescriptor::QuestsMissionsKeyItems => {
                converter.use_dat(DatIdMapping::get().quests_mission_keyitems.clone())
            }

            DatDescriptor::AbilityNames => {
                converter.use_dat(DatIdMapping::get().ability_names.clone())
            }
            DatDescriptor::AbilityDescriptions => {
                converter.use_dat(DatIdMapping::get().ability_descriptions.clone())
            }
            DatDescriptor::AreaNames => converter.use_dat(DatIdMapping::get().area_names.clone()),
            DatDescriptor::AreaNamesShort => {
                converter.use_dat(DatIdMapping::get().area_names_short.clone())
            }
            DatDescriptor::AreaNamesAlt => {
                converter.use_dat(DatIdMapping::get().area_names_alt.clone())
            }
            DatDescriptor::Augments => converter.use_dat(DatIdMapping::get().augments.clone()),
            DatDescriptor::BlueMagic => converter.use_dat(DatIdMapping::get().blue_magic.clone()),
            DatDescriptor::CallMount => converter.use_dat(DatIdMapping::get().call_mount.clone()),
            DatDescriptor::CharacterSelect => {
                converter.use_dat(DatIdMapping::get().character_select.clone())
            }
            DatDescriptor::ChatFilterTypes => {
                converter.use_dat(DatIdMapping::get().chat_filter_types.clone())
            }
            DatDescriptor::ChocoboNames => {
                converter.use_dat(DatIdMapping::get().chocobo_names.clone())
            }
            DatDescriptor::CommandUsage => {
                converter.use_dat(DatIdMapping::get().command_usage.clone())
            }
            DatDescriptor::EinherjarChambers => {
                converter.use_dat(DatIdMapping::get().einherjar_chambers.clone())
            }
            DatDescriptor::Emotes => converter.use_dat(DatIdMapping::get().emotes.clone()),
            DatDescriptor::DayNames => converter.use_dat(DatIdMapping::get().day_names.clone()),
            DatDescriptor::Directions => converter.use_dat(DatIdMapping::get().directions.clone()),
            DatDescriptor::EquipmentLocations => {
                converter.use_dat(DatIdMapping::get().equipment_locations.clone())
            }
            DatDescriptor::EquipmentLocationsAlt => {
                converter.use_dat(DatIdMapping::get().equipment_locations_alt.clone())
            }
            DatDescriptor::ErrorMessages => {
                converter.use_dat(DatIdMapping::get().error_messages.clone())
            }
            DatDescriptor::IngameMessages1 => {
                converter.use_dat(DatIdMapping::get().ingame_messages_1.clone())
            }
            DatDescriptor::IngameMessages2 => {
                converter.use_dat(DatIdMapping::get().ingame_messages_2.clone())
            }
            DatDescriptor::JobNames => converter.use_dat(DatIdMapping::get().job_names.clone()),
            DatDescriptor::JobNamesShort => {
                converter.use_dat(DatIdMapping::get().job_names_short.clone())
            }
            DatDescriptor::JobPointBonuses => {
                converter.use_dat(DatIdMapping::get().job_point_bonuses.clone())
            }
            DatDescriptor::JobPointGifts => {
                converter.use_dat(DatIdMapping::get().job_point_gifts.clone())
            }
            DatDescriptor::KeyItems => converter.use_dat(DatIdMapping::get().key_items.clone()),
            DatDescriptor::MenuItemsDescription => {
                converter.use_dat(DatIdMapping::get().menu_items_description.clone())
            }
            DatDescriptor::MenuItemsText => {
                converter.use_dat(DatIdMapping::get().menu_items_text.clone())
            }
            DatDescriptor::Merits => converter.use_dat(DatIdMapping::get().merits.clone()),
            DatDescriptor::MissionsAcp => {
                converter.use_dat(DatIdMapping::get().missions_acp.clone())
            }
            DatDescriptor::MissionsAmke => {
                converter.use_dat(DatIdMapping::get().missions_amke.clone())
            }
            DatDescriptor::MissionsAsa => {
                converter.use_dat(DatIdMapping::get().missions_asa.clone())
            }
            DatDescriptor::MissionsAssault => {
                converter.use_dat(DatIdMapping::get().missions_assault.clone())
            }
            DatDescriptor::MissionsBastok => {
                converter.use_dat(DatIdMapping::get().missions_bastok.clone())
            }
            DatDescriptor::MissionsCampaign => {
                converter.use_dat(DatIdMapping::get().missions_campaign.clone())
            }
            DatDescriptor::MissionsCop => {
                converter.use_dat(DatIdMapping::get().missions_cop.clone())
            }
            DatDescriptor::MissionsRov => {
                converter.use_dat(DatIdMapping::get().missions_rov.clone())
            }
            DatDescriptor::MissionsSandoria => {
                converter.use_dat(DatIdMapping::get().missions_sandoria.clone())
            }
            DatDescriptor::MissionsSoa => {
                converter.use_dat(DatIdMapping::get().missions_soa.clone())
            }
            DatDescriptor::MissionsToau => {
                converter.use_dat(DatIdMapping::get().missions_toau.clone())
            }
            DatDescriptor::MissionsWindurst => {
                converter.use_dat(DatIdMapping::get().missions_windurst.clone())
            }
            DatDescriptor::MissionsWotg => {
                converter.use_dat(DatIdMapping::get().missions_wotg.clone())
            }
            DatDescriptor::MissionsZilart => {
                converter.use_dat(DatIdMapping::get().missions_zilart.clone())
            }
            DatDescriptor::MoblinMazeMongers => {
                converter.use_dat(DatIdMapping::get().moblin_maze_mongers.clone())
            }
            DatDescriptor::Modifiers => converter.use_dat(DatIdMapping::get().modifiers.clone()),
            DatDescriptor::MonsterFamilies => {
                converter.use_dat(DatIdMapping::get().monster_families.clone())
            }
            DatDescriptor::MoonPhases => converter.use_dat(DatIdMapping::get().moon_phases.clone()),
            DatDescriptor::PankrationNames => {
                converter.use_dat(DatIdMapping::get().pankration_names.clone())
            }
            DatDescriptor::PolMessages => {
                converter.use_dat(DatIdMapping::get().pol_messages.clone())
            }
            DatDescriptor::QuestsAbyssea => {
                converter.use_dat(DatIdMapping::get().quests_abyssea.clone())
            }
            DatDescriptor::QuestsBastok => {
                converter.use_dat(DatIdMapping::get().quests_bastok.clone())
            }
            DatDescriptor::QuestsCoalition => {
                converter.use_dat(DatIdMapping::get().quests_coalition.clone())
            }
            DatDescriptor::QuestsJeuno => {
                converter.use_dat(DatIdMapping::get().quests_jeuno.clone())
            }
            DatDescriptor::QuestsOther => {
                converter.use_dat(DatIdMapping::get().quests_other.clone())
            }
            DatDescriptor::QuestsOutlands => {
                converter.use_dat(DatIdMapping::get().quests_outlands.clone())
            }
            DatDescriptor::QuestsSandoria => {
                converter.use_dat(DatIdMapping::get().quests_sandoria.clone())
            }
            DatDescriptor::QuestsSoa => converter.use_dat(DatIdMapping::get().quests_soa.clone()),
            DatDescriptor::QuestsToau => converter.use_dat(DatIdMapping::get().quests_toau.clone()),
            DatDescriptor::QuestsWindurst => {
                converter.use_dat(DatIdMapping::get().quests_windurst.clone())
            }
            DatDescriptor::QuestsWotg => converter.use_dat(DatIdMapping::get().quests_wotg.clone()),
            DatDescriptor::RaceNames => converter.use_dat(DatIdMapping::get().race_names.clone()),
            DatDescriptor::RegionNames => {
                converter.use_dat(DatIdMapping::get().region_names.clone())
            }
            DatDescriptor::ServerNames => {
                converter.use_dat(DatIdMapping::get().server_names.clone())
            }
            DatDescriptor::SpellNames => converter.use_dat(DatIdMapping::get().spell_names.clone()),
            DatDescriptor::SpellDescriptions => {
                converter.use_dat(DatIdMapping::get().spell_descriptions.clone())
            }
            DatDescriptor::StatusInfo => converter.use_dat(DatIdMapping::get().status_info.clone()),
            DatDescriptor::StatusNames => {
                converter.use_dat(DatIdMapping::get().status_names.clone())
            }
            DatDescriptor::TimeAndPronouns => {
                converter.use_dat(DatIdMapping::get().time_and_pronouns.clone())
            }
            DatDescriptor::Titles => converter.use_dat(DatIdMapping::get().titles.clone()),
            DatDescriptor::TrustMessages => {
                converter.use_dat(DatIdMapping::get().trust_messages.clone())
            }
            DatDescriptor::Misc1 => converter.use_dat(DatIdMapping::get().misc1.clone()),
            DatDescriptor::Misc2 => converter.use_dat(DatIdMapping::get().misc2.clone()),
            DatDescriptor::WeatherTypes => {
                converter.use_dat(DatIdMapping::get().weather_types.clone())
            }

            DatDescriptor::Armor => converter.use_dat(DatIdMapping::get().armor.clone()),
            DatDescriptor::Armor2 => converter.use_dat(DatIdMapping::get().armor2.clone()),
            DatDescriptor::Currency => converter.use_dat(DatIdMapping::get().currency.clone()),
            DatDescriptor::GeneralItems => {
                converter.use_dat(DatIdMapping::get().general_items.clone())
            }
            DatDescriptor::GeneralItems2 => {
                converter.use_dat(DatIdMapping::get().general_items2.clone())
            }
            DatDescriptor::PuppetItems => {
                converter.use_dat(DatIdMapping::get().puppet_items.clone())
            }
            DatDescriptor::UsableItems => {
                converter.use_dat(DatIdMapping::get().usable_items.clone())
            }
            DatDescriptor::Weapons => converter.use_dat(DatIdMapping::get().weapons.clone()),
            DatDescriptor::VouchersAndSlips => {
                converter.use_dat(DatIdMapping::get().vouchers_and_slips.clone())
            }
            DatDescriptor::Monipulator => {
                converter.use_dat(DatIdMapping::get().monipulator.clone())
            }
            DatDescriptor::Instincts => converter.use_dat(DatIdMapping::get().instincts.clone()),

            // Global dialog
            DatDescriptor::MonsterSkillNames => {
                converter.use_dat(DatIdMapping::get().monster_skill_names.clone())
            }
            DatDescriptor::StatusNamesDialog => {
                converter.use_dat(DatIdMapping::get().status_names_dialog.clone())
            }
            DatDescriptor::EmoteMessages => {
                converter.use_dat(DatIdMapping::get().emote_messages.clone())
            }
            DatDescriptor::SystemMessages1 => {
                converter.use_dat(DatIdMapping::get().system_messages_1.clone())
            }
            DatDescriptor::SystemMessages2 => {
                converter.use_dat(DatIdMapping::get().system_messages_2.clone())
            }
            DatDescriptor::SystemMessages3 => {
                converter.use_dat(DatIdMapping::get().system_messages_3.clone())
            }
            DatDescriptor::SystemMessages4 => {
                converter.use_dat(DatIdMapping::get().system_messages_4.clone())
            }
            DatDescriptor::UnityDialogs => {
                converter.use_dat(DatIdMapping::get().unity_dialogs.clone())
            }

            // By zone
            DatDescriptor::ZoneData(zone_id) => {
                converter.use_dat(DatIdMapping::get().zone_data.get_result(&zone_id)?.clone())
            }
            DatDescriptor::EntityNames(zone_id) => {
                converter.use_dat(DatIdMapping::get().entities.get_result(&zone_id)?.clone())
            }
            DatDescriptor::Dialog(zone_id) => {
                converter.use_dat(DatIdMapping::get().dialog.get_result(&zone_id)?.clone())
            }
            DatDescriptor::Dialog2(zone_id) => {
                converter.use_dat(DatIdMapping::get().dialog.get_result(&zone_id)?.clone())
            }
            DatDescriptor::Events(zone_id) => {
                converter.use_dat(DatIdMapping::get().events.get_result(&zone_id)?.clone())
            }
        }
    }
}
