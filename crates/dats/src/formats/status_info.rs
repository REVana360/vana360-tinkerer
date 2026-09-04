use anyhow::{Result, anyhow};
use common::{
    byte_walker::{BufferedByteWalker, ByteWalker},
    vec_byte_walker::VecByteWalker,
    writing_byte_walker::WritingByteWalker,
};
use encoding::{decoder::Decoder, encoder::Encoder};
use serde_derive::{Deserialize, Serialize};

use crate::serde_base64;
use crate::{
    dat_format::DatFormat,
    enums::{StatusEffectCancellable, StatusEffectSystem},
    utils::{decode_data_block, encode_data_block},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfo {
    id: u16,
    description: String,

    cancellable: StatusEffectCancellable,
    system: StatusEffectSystem,

    #[serde(with = "serde_base64")]
    icon_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StatusInfoLayout {
    Legacy,
    Modern,
}

impl StatusInfoLayout {
    fn entry_size(self) -> usize {
        match self {
            Self::Legacy => 0xC00,
            Self::Modern => 0x1800,
        }
    }
}

impl StatusInfo {
    fn parse<T: ByteWalker>(walker: &mut T, layout: StatusInfoLayout) -> Result<StatusInfo> {
        let entry_start = walker.offset();
        let mut data_bytes = walker.take_bytes(0x280)?.to_vec();
        decode_data_block(&mut data_bytes);

        let mut data_walker = BufferedByteWalker::on(data_bytes);

        let id = data_walker.step::<u16>()?;

        let cancellable = StatusEffectCancellable::from(data_walker.step::<u8>()?);
        let system = StatusEffectSystem::from(data_walker.step::<u8>()?);

        data_walker.expect::<u32>(1)?;
        data_walker.expect::<u32>(12)?;
        data_walker.expect::<u32>(0)?;
        data_walker.expect::<u32>(1)?;

        data_walker.expect_n_msg::<u8>(0, 24, "Padding after unknowns")?;

        let description = Decoder::decode_simple(data_walker.step_until(0)?)?;

        let icon_size = walker.step::<u32>()?;
        let icon_bytes = walker.take_bytes(icon_size as usize)?.to_vec();

        let entry_end = entry_start + layout.entry_size();
        let icon_padding = entry_end
            .checked_sub(walker.offset() + 1)
            .ok_or_else(|| anyhow!("Status icon exceeds its entry boundary."))?;
        walker.expect_n_msg::<u8>(0, icon_padding, "Padding after icon")?;
        walker.expect_msg::<u8>(0xFF, "End of status info byte")?;

        Ok(StatusInfo {
            id,
            cancellable,
            system,
            description,
            icon_bytes,
        })
    }

    fn write<T: WritingByteWalker>(&self, walker: &mut T, layout: StatusInfoLayout) -> Result<()> {
        let entry_start = walker.offset();
        let mut data_walker = VecByteWalker::with_size(0x280);

        data_walker.write(self.id);
        data_walker.write::<u8>(self.cancellable.into());
        data_walker.write::<u8>(self.system.into());

        data_walker.write::<u32>(1);
        data_walker.write::<u32>(12);
        data_walker.write::<u32>(0);
        data_walker.write::<u32>(1);

        data_walker.skip(24);

        let description = Encoder::encode_simple(&self.description)?;
        data_walker.write_bytes(&description);

        let mut data_bytes = data_walker.into_vec();
        encode_data_block(&mut data_bytes);

        walker.write_bytes(&data_bytes);

        walker.write(self.icon_bytes.len() as u32);
        walker.write_bytes(&self.icon_bytes);

        let entry_end = entry_start + layout.entry_size();
        let icon_padding = entry_end
            .checked_sub(walker.offset() + 1)
            .ok_or_else(|| anyhow!("Status icon exceeds its entry boundary."))?;
        walker.skip(icon_padding);

        walker.write::<u8>(0xFF);

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusInfoTable {
    layout: StatusInfoLayout,
    status_infos: Vec<StatusInfo>,
}

impl StatusInfoTable {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let bytes = walker.take_bytes(walker.remaining())?;
        let mut parsed = Vec::new();
        let mut errors = Vec::new();

        for layout in [StatusInfoLayout::Legacy, StatusInfoLayout::Modern] {
            if bytes.len() % layout.entry_size() != 0 {
                continue;
            }

            match Self::parse_with_layout(bytes, layout) {
                Ok(table) => parsed.push(table),
                Err(error) => errors.push(format!("{layout:?}: {error}")),
            }
        }

        match parsed.len() {
            1 => Ok(parsed.pop().unwrap()),
            0 => Err(anyhow!(
                "Status info length {} does not match a supported layout: {}",
                bytes.len(),
                errors.join("; ")
            )),
            _ => Err(anyhow!(
                "Status info length {} matches multiple supported layouts.",
                bytes.len()
            )),
        }
    }

    fn parse_with_layout(bytes: &[u8], layout: StatusInfoLayout) -> Result<Self> {
        let mut walker = BufferedByteWalker::on(bytes);
        let entry_count = bytes.len() / layout.entry_size();
        let mut status_infos = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            status_infos.push(StatusInfo::parse(&mut walker, layout)?);
        }

        Ok(StatusInfoTable {
            layout,
            status_infos,
        })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.set_size(self.status_infos.len() * self.layout.entry_size());

        for status_info in &self.status_infos {
            status_info.write(walker, self.layout)?;
        }

        Ok(())
    }
}

impl DatFormat for StatusInfoTable {
    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        StatusInfoTable::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        StatusInfoTable::parse(walker).map(|_| ())
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
        enums::{StatusEffectCancellable, StatusEffectSystem},
    };

    use super::{StatusInfo, StatusInfoLayout, StatusInfoTable};

    #[test]
    pub fn status_infos() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/status_infos.DAT");

        StatusInfoTable::check_path(&dat_path).unwrap();
        let res = StatusInfoTable::from_path_checked_yaml(&dat_path).unwrap();

        assert_eq!(res.layout, StatusInfoLayout::Modern);
        assert_eq!(
            res.status_infos[0].description,
            "You have been knocked unconscious.".to_string()
        );
        assert_eq!(res.status_infos[0].cancellable, StatusEffectCancellable::No);
        assert_eq!(
            res.status_infos[0].system,
            StatusEffectSystem::NoTimerWarning
        );

        assert_eq!(res.status_infos[1].cancellable, StatusEffectCancellable::No);
        assert_eq!(res.status_infos[1].system, StatusEffectSystem::Normal);

        assert_eq!(
            res.status_infos[32].cancellable,
            StatusEffectCancellable::FromMenu
        );
        assert_eq!(res.status_infos[32].system, StatusEffectSystem::Normal);

        assert_eq!(
            res.status_infos[614].description,
            "Ullegore is making you forget the true meaning of \"fun\"!".to_string()
        );
    }

    #[test]
    fn legacy_status_info_round_trip() {
        let table = StatusInfoTable {
            layout: StatusInfoLayout::Legacy,
            status_infos: vec![StatusInfo {
                id: 7,
                description: "Synthetic legacy status".to_owned(),
                cancellable: StatusEffectCancellable::FromMenu,
                system: StatusEffectSystem::Normal,
                icon_bytes: vec![1, 2, 3, 4],
            }],
        };

        let bytes = table.to_bytes().unwrap();
        assert_eq!(bytes.len(), 0xC00);

        let parsed = StatusInfoTable::from_bytes_checked(&bytes).unwrap();
        assert_eq!(parsed.layout, StatusInfoLayout::Legacy);
        assert_eq!(parsed.status_infos[0].id, 7);

        let yaml = serde_yaml::to_string(&parsed).unwrap();
        let parsed_from_yaml: StatusInfoTable = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed_from_yaml.to_bytes().unwrap(), bytes);
    }
}
