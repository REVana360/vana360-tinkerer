use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc::Sender},
};

use dats::{
    context::DatContext,
    id_mapping::{DatDescriptor, DatLanguage},
};
use serde::{Deserialize, Serialize};
use threadpool::ThreadPool;

use crate::dat_yaml_util::DatYamlUtil;

#[derive(Debug)]
pub struct DatProcessor {
    tx: Sender<DatProcessorMessage>,
    pub is_preprocessing: bool,
    pool: Mutex<ThreadPool>,
}

#[derive(Debug, Clone, specta::Type, Serialize, Deserialize)]
pub struct DatProcessorMessage {
    pub dat_descriptor: DatDescriptor,
    pub output_kind: DatProcessorOutputKind,
    pub state: DatProcessingState,
}

#[derive(Debug, Clone, specta::Type, Serialize, Deserialize)]
pub enum DatProcessorOutputKind {
    Dat,
    Yaml,
}

#[derive(Debug, Clone, specta::Type, Serialize, Deserialize)]
pub enum DatProcessingState {
    Working,
    Finished(PathBuf),
    Error(String),
}

impl DatProcessor {
    pub fn new(tx: Sender<DatProcessorMessage>) -> Self {
        Self {
            tx,
            is_preprocessing: false,
            pool: Mutex::new(
                threadpool::Builder::new()
                    .thread_name("dat-processor".to_string())
                    .build(),
            ),
        }
    }

    pub fn dat_to_yaml(
        &self,
        dat_descriptor: DatDescriptor,
        lang: DatLanguage,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
    ) {
        let tx = self.tx.clone();
        let start_message = DatProcessorMessage {
            dat_descriptor,
            output_kind: DatProcessorOutputKind::Yaml,
            state: DatProcessingState::Working,
        };
        if let Err(err) = tx.send(start_message) {
            eprintln!("Failed to notify about DAT to YAML start: {err}");
        }

        self.pool.lock().unwrap().execute(move || {
            let res =
                DatYamlUtil::dat_to_yaml(&dat_descriptor, lang, dat_context, raw_data_root_path)
                    .map(|path| DatProcessorMessage {
                        dat_descriptor,
                        output_kind: DatProcessorOutputKind::Yaml,
                        state: DatProcessingState::Finished(path),
                    })
                    .unwrap_or_else(|err| DatProcessorMessage {
                        dat_descriptor,
                        output_kind: DatProcessorOutputKind::Yaml,
                        state: DatProcessingState::Error(err.to_string()),
                    });

            if let Err(err) = tx.send(res) {
                eprintln!("Failed to notify about DAT to YAML result: {err}");
            }
        });
    }

    pub fn yaml_to_dat(
        &self,
        dat_descriptor: DatDescriptor,
        lang: DatLanguage,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
        dat_root_path: PathBuf,
    ) {
        let tx = self.tx.clone();
        let start_message = DatProcessorMessage {
            dat_descriptor,
            output_kind: DatProcessorOutputKind::Dat,
            state: DatProcessingState::Working,
        };
        if let Err(err) = tx.send(start_message) {
            eprintln!("Failed to notify about YAML to DAT start: {err}");
        }

        self.pool.lock().unwrap().execute(move || {
            let res: DatProcessorMessage = DatYamlUtil::yaml_to_dat(
                &dat_descriptor,
                lang,
                dat_context,
                raw_data_root_path,
                dat_root_path,
            )
            .map(|path| DatProcessorMessage {
                dat_descriptor,
                output_kind: DatProcessorOutputKind::Dat,
                state: DatProcessingState::Finished(path),
            })
            .unwrap_or_else(|err| DatProcessorMessage {
                dat_descriptor,
                output_kind: DatProcessorOutputKind::Dat,
                state: DatProcessingState::Error(err.to_string()),
            });

            if let Err(err) = tx.send(res) {
                eprintln!("Failed to notify about YAML to DAT result: {err}");
            }
        });
    }

    pub fn all_yaml_to_dats(
        &mut self,
        dat_context: Arc<DatContext>,
        in_dir: &PathBuf,
        out_dir: &PathBuf,
    ) -> usize {
        self.is_preprocessing = true;

        let mut count = 0;

        walkdir::WalkDir::new(&in_dir)
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.file_type().is_dir() {
                    return None;
                }

                let path = entry.into_path();
                let dat = DatYamlUtil::dat_from_path(&path, &in_dir, &dat_context);

                if dat.is_none() {
                    eprintln!(
                        "Could not map the following file to a DAT: {}",
                        path.to_string_lossy()
                    );
                }
                dat
            })
            .for_each(|dat| {
                let dat_context: Arc<DatContext> = dat_context.clone();
                let raw_data_root_path = in_dir.clone();
                let dat_root_path = out_dir.clone();

                count += 1;
                self.yaml_to_dat(
                    dat.descriptor,
                    dat.lang,
                    dat_context,
                    raw_data_root_path,
                    dat_root_path,
                );
            });

        self.is_preprocessing = false;
        count
    }
}
