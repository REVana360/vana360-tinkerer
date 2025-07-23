use anyhow::{Result, anyhow};
use dats::{
    base::{DatByZone, ZoneId},
    context::DatContext,
    dat_format::DatFormat,
};
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct ZoneInfo {
    pub id: ZoneId,
    pub name: String,
}

pub async fn get_zone_ids_from_dats<T>(
    dat_by_zone: &DatByZone<T>,
    dat_context: &DatContext,
) -> Result<Vec<ZoneInfo>>
where
    T: DatFormat + 'static,
{
    let mut join_set: JoinSet<Result<ZoneInfo>> = JoinSet::new();

    dat_by_zone.map.iter().for_each(|(zone_id, dat_id)| {
        let zone_id = zone_id.clone();
        let dat_id = dat_id.clone();
        let dat_context = dat_context.clone();

        join_set.spawn(async move {
            let zone_name = dat_context
                .zone_id_to_name
                .get(&zone_id)
                .ok_or(anyhow!("No zone name for ID: {zone_id}."))?;

            match dat_context.check_dat(&dat_id) {
                Ok(_) => Ok::<ZoneInfo, anyhow::Error>(ZoneInfo {
                    id: zone_id.clone(),
                    name: zone_name.display_name.clone(),
                }),
                Err(err) => Err(err.into()),
            }
        });
    });

    let mut result = vec![];
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(res)) => result.push(res),
            Ok(Err(_)) => {}
            Err(err) => {
                eprintln!("Join error: {err}");
            }
        }
    }

    Ok(result)
}
