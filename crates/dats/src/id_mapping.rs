use std::sync::OnceLock;

use crate::{
    base::{Dat, DatByZone},
    formats::{
        dialog::Dialog, dmsg_table::DmsgTable, entity_names::EntityNames, events::Events,
        item_info::ItemInfoTable, menu_table::MenuTable, status_info::StatusInfoTable,
        xistring_table::XiStringTable, zone_data::ZoneData,
    },
};

#[derive(Debug)]
pub struct DatIdMapping {
    // Zone data
    pub zone_data: DatByZone<ZoneData>,
    pub entities: DatByZone<EntityNames>,
    pub dialog: DatByZone<Dialog>,
    pub dialog2: DatByZone<Dialog>,
    pub events: DatByZone<Events>,

    // Global dialog
    pub monster_skill_names: Dat<Dialog>,
    pub status_names_dialog: Dat<Dialog>,
    pub emote_messages: Dat<Dialog>,
    pub system_messages_1: Dat<Dialog>,
    pub system_messages_2: Dat<Dialog>,
    pub system_messages_3: Dat<Dialog>,
    pub system_messages_4: Dat<Dialog>,
    pub unity_dialogs: Dat<Dialog>,

    // String tables
    pub ability_names: Dat<DmsgTable>,
    pub ability_descriptions: Dat<DmsgTable>,
    pub area_names: Dat<DmsgTable>,
    pub area_names_short: Dat<DmsgTable>,
    pub area_names_alt: Dat<DmsgTable>,
    pub augments: Dat<DmsgTable>,
    pub blue_magic: Dat<DmsgTable>,
    pub call_mount: Dat<DmsgTable>,
    pub character_select: Dat<DmsgTable>,
    pub chat_filter_types: Dat<DmsgTable>,
    pub chocobo_names: Dat<DmsgTable>,
    pub command_usage: Dat<DmsgTable>,
    pub day_names: Dat<DmsgTable>,
    pub directions: Dat<DmsgTable>,
    pub einherjar_chambers: Dat<DmsgTable>,
    pub emotes: Dat<DmsgTable>,
    pub equipment_locations: Dat<DmsgTable>,
    pub equipment_locations_alt: Dat<DmsgTable>,
    pub error_messages: Dat<DmsgTable>,
    pub ingame_messages_1: Dat<DmsgTable>,
    pub ingame_messages_2: Dat<XiStringTable>,
    pub job_names: Dat<DmsgTable>,
    pub job_names_short: Dat<DmsgTable>,
    pub job_point_bonuses: Dat<DmsgTable>,
    pub job_point_gifts: Dat<DmsgTable>,
    pub key_items: Dat<DmsgTable>,
    pub menu_items_description: Dat<DmsgTable>,
    pub menu_items_text: Dat<DmsgTable>,
    pub merits: Dat<DmsgTable>,
    pub missions_acp: Dat<DmsgTable>,
    pub missions_amke: Dat<DmsgTable>,
    pub missions_asa: Dat<DmsgTable>,
    pub missions_assault: Dat<DmsgTable>,
    pub missions_bastok: Dat<DmsgTable>,
    pub missions_campaign: Dat<DmsgTable>,
    pub missions_cop: Dat<DmsgTable>,
    pub missions_rov: Dat<DmsgTable>,
    pub missions_sandoria: Dat<DmsgTable>,
    pub missions_soa: Dat<DmsgTable>,
    pub missions_toau: Dat<DmsgTable>,
    pub missions_windurst: Dat<DmsgTable>,
    pub missions_wotg: Dat<DmsgTable>,
    pub missions_zilart: Dat<DmsgTable>,
    pub moblin_maze_mongers: Dat<DmsgTable>,
    pub modifiers: Dat<DmsgTable>,
    pub monster_families: Dat<DmsgTable>,
    pub moon_phases: Dat<DmsgTable>,
    pub mount_names: Dat<DmsgTable>,
    pub pankration_names: Dat<DmsgTable>,
    pub pol_messages: Dat<XiStringTable>,
    pub quests_abyssea: Dat<DmsgTable>,
    pub quests_bastok: Dat<DmsgTable>,
    pub quests_coalition: Dat<DmsgTable>,
    pub quests_jeuno: Dat<DmsgTable>,
    pub quests_other: Dat<DmsgTable>,
    pub quests_outlands: Dat<DmsgTable>,
    pub quests_sandoria: Dat<DmsgTable>,
    pub quests_soa: Dat<DmsgTable>,
    pub quests_toau: Dat<DmsgTable>,
    pub quests_windurst: Dat<DmsgTable>,
    pub quests_wotg: Dat<DmsgTable>,
    pub race_names: Dat<DmsgTable>,
    pub region_names: Dat<DmsgTable>,
    pub server_names: Dat<DmsgTable>,
    pub spell_names: Dat<DmsgTable>,
    pub spell_descriptions: Dat<DmsgTable>,
    pub status_info: Dat<StatusInfoTable>,
    pub status_names: Dat<DmsgTable>,
    pub time_and_pronouns: Dat<XiStringTable>,
    pub titles: Dat<DmsgTable>,
    pub trust_messages: Dat<DmsgTable>,
    pub misc1: Dat<DmsgTable>,
    pub misc2: Dat<DmsgTable>,
    pub weather_types: Dat<DmsgTable>,

    // Item data
    pub armor: Dat<ItemInfoTable>,
    pub armor2: Dat<ItemInfoTable>,
    pub currency: Dat<ItemInfoTable>,
    pub general_items: Dat<ItemInfoTable>,
    pub general_items2: Dat<ItemInfoTable>,
    pub puppet_items: Dat<ItemInfoTable>,
    pub usable_items: Dat<ItemInfoTable>,
    pub weapons: Dat<ItemInfoTable>,
    pub vouchers_and_slips: Dat<ItemInfoTable>,
    pub monipulator: Dat<ItemInfoTable>,
    pub instincts: Dat<ItemInfoTable>,

    // Misc data
    pub data_menu: Dat<MenuTable>,
    pub quests_mission_keyitems: Dat<MenuTable>,
}

static DAT_ID_MAPPING: OnceLock<DatIdMapping> = OnceLock::new();

impl DatIdMapping {
    pub fn get() -> &'static Self {
        DAT_ID_MAPPING.get_or_init(|| {
            // Zone data
            let mut zone_data = DatByZone::default();
            // Zones 0-255
            (0..256).into_iter().for_each(|idx| {
                zone_data.insert(idx, 100 + idx);
            });
            // Zones 256-512
            (0..256).into_iter().for_each(|idx| {
                zone_data.insert(idx + 256, 83891 + idx);
            });

            // Entities
            let mut entities = DatByZone::default();
            // Zones 1-255
            (0..256).into_iter().for_each(|idx| {
                entities.insert(idx, 6720 + idx);
            });
            // Zones 256-512
            (0..256).into_iter().for_each(|idx| {
                entities.insert(256 + idx, 86491 + idx);
            });
            // Zones 1000+
            (0..256).into_iter().for_each(|idx| {
                entities.insert(1000 + idx, 67911 + idx);
            });

            // Dialog text
            let mut dialog = DatByZone::default();
            // Zones 0-255
            (0..256).into_iter().for_each(|idx| {
                dialog.insert(idx, 6420 + idx);
            });
            // Zones 256-512
            (0..256).into_iter().for_each(|idx| {
                dialog.insert(idx + 256, 85590 + idx);
            });

            // Events
            // Zones 0-255
            let mut events = DatByZone::default();
            (0..256).into_iter().for_each(|idx| {
                events.insert(idx, 5820 + idx);
            });
            // Zones 256-512
            (0..256).into_iter().for_each(|idx| {
                events.insert(idx + 256, 84991 + idx);
            });

            // Secondary dialog text
            let mut dialog2 = DatByZone::default();
            // Just whitegate?
            dialog2.insert(50, 57945);

            Self {
                zone_data,
                entities,
                dialog,
                dialog2,
                events,

                // Global dialog
                monster_skill_names: 07035.into(),
                status_names_dialog: 07029.into(),
                emote_messages: 07025.into(),
                system_messages_1: 07023.into(),
                system_messages_2: 07031.into(),
                system_messages_3: 07021.into(),
                system_messages_4: 07027.into(),
                unity_dialogs: 07039.into(),

                // String tables
                ability_names: 55701.into(),
                ability_descriptions: 55733.into(),
                area_names: 55465.into(),
                area_names_short: 55466.into(),
                area_names_alt: 55661.into(),
                augments: 55692.into(),
                blue_magic: 55685.into(),
                call_mount: 55682.into(),
                character_select: 55470.into(),
                chat_filter_types: 55650.into(),
                chocobo_names: 55474.into(),
                command_usage: 55687.into(),
                day_names: 55658.into(),
                directions: 55659.into(),
                einherjar_chambers: 55472.into(),
                emotes: 55675.into(),
                equipment_locations: 55471.into(),
                equipment_locations_alt: 55666.into(),
                error_messages: 55646.into(),
                ingame_messages_1: 55648.into(),
                ingame_messages_2: 55649.into(),
                job_names: 55467.into(),
                job_names_short: 55468.into(),
                job_point_bonuses: 55694.into(),
                job_point_gifts: 55674.into(),
                key_items: 55695.into(),
                menu_items_description: 55651.into(),
                menu_items_text: 55652.into(),
                merits: 55686.into(),
                missions_acp: 55735.into(),
                missions_amke: 55736.into(),
                missions_asa: 55737.into(),
                missions_assault: 55720.into(),
                missions_bastok: 55716.into(),
                missions_campaign: 55724.into(),
                missions_cop: 55719.into(),
                missions_rov: 55741.into(),
                missions_sandoria: 55715.into(),
                missions_soa: 55738.into(),
                missions_toau: 55721.into(),
                missions_windurst: 55717.into(),
                missions_wotg: 55723.into(),
                missions_zilart: 55718.into(),
                moblin_maze_mongers: 55691.into(),
                modifiers: 55689.into(),
                monster_families: 55690.into(),
                moon_phases: 55660.into(),
                mount_names: 55681.into(),
                pankration_names: 55473.into(),
                pol_messages: 55647.into(),
                quests_abyssea: 55713.into(),
                quests_bastok: 55707.into(),
                quests_coalition: 55740.into(),
                quests_jeuno: 55709.into(),
                quests_other: 55710.into(),
                quests_outlands: 55711.into(),
                quests_sandoria: 55706.into(),
                quests_soa: 55739.into(),
                quests_toau: 55712.into(),
                quests_windurst: 55708.into(),
                quests_wotg: 55722.into(),
                race_names: 55469.into(),
                region_names: 55654.into(),
                server_names: 55680.into(),
                spell_names: 55702.into(),
                spell_descriptions: 55734.into(),
                status_info: 00087.into(),
                status_names: 55725.into(),
                time_and_pronouns: 00063.into(),
                titles: 55704.into(),
                trust_messages: 55693.into(),
                misc1: 55645.into(),
                misc2: 55653.into(),
                weather_types: 55657.into(),

                // Item data
                armor: 00076.into(),
                armor2: 55668.into(),
                currency: 00091.into(),
                general_items: 00073.into(),
                general_items2: 55671.into(),
                puppet_items: 00077.into(),
                usable_items: 00074.into(),
                weapons: 00075.into(),
                vouchers_and_slips: 55667.into(),
                monipulator: 55669.into(),
                instincts: 55670.into(),

                // Misc. data
                data_menu: 81.into(),
                quests_mission_keyitems: 82.into(),
            }
        })
    }
}
