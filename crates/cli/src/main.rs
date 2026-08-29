mod analyze_meshes;
mod audit_dats;
mod export_client_globals;
mod export_dat;
mod export_items;
mod export_ximesh;
mod export_zone_entities;
mod export_zone_text;
mod make_dats;
mod scan_dats;
mod util;

use std::path::PathBuf;

use analyze_meshes::analyze_zone_meshes;
use anyhow::Result;
use audit_dats::audit_dats;
use clap::{Parser, Subcommand};
use export_client_globals::export_client_globals;
use export_items::export_items;
use export_zone_entities::export_zone_entities;
use export_zone_text::export_zone_text;

use dats::base::DatId;
use export_ximesh::export_zone_meshes;
use make_dats::make_dats;

use crate::{export_dat::export_dat, scan_dats::scan_dats};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    ExportZoneMesh {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: String,

        #[arg(value_name = "OUT_DIR")]
        out_dir: Option<String>,
    },

    AnalyzeZoneMesh {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: String,
    },

    MakeDats {
        #[arg(value_name = "PROJECT_DIR")]
        project_dir: PathBuf,

        #[arg(value_name = "YAML_FILES")]
        yaml_files: Vec<PathBuf>,

        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    ScanDats {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,
    },

    AuditDats {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,

        #[arg(short, long, value_name = "JSON_OUTPUT")]
        out: Option<PathBuf>,

        #[arg(long)]
        xbox_packages: bool,
    },

    ExportClientGlobals {
        #[arg(value_name = "RUNTIME_ROOT")]
        runtime_root: PathBuf,

        #[arg(short, long, value_name = "JSON_OUTPUT")]
        out: PathBuf,
    },

    ExportItems {
        #[arg(value_name = "RUNTIME_ROOT")]
        runtime_root: PathBuf,

        #[arg(short, long, value_name = "JSON_OUTPUT")]
        out: PathBuf,
    },

    ExportZoneText {
        #[arg(value_name = "RUNTIME_ROOT")]
        runtime_root: PathBuf,

        #[arg(short, long, value_name = "JSON_OUTPUT")]
        out: PathBuf,
    },

    ExportZoneEntities {
        #[arg(value_name = "RUNTIME_ROOT")]
        runtime_root: PathBuf,

        #[arg(short, long, value_name = "JSON_OUTPUT")]
        out: PathBuf,
    },

    ExportDat {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,

        #[arg(long)]
        dat_path: Option<PathBuf>,

        #[arg(long)]
        dat_id: Option<u32>,

        #[arg(value_name = "OUT_PATH")]
        out_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::ExportZoneMesh { ffxi_path, out_dir } => {
            export_zone_meshes(
                PathBuf::from(ffxi_path),
                PathBuf::from(out_dir.unwrap_or(".".to_string())),
            )
            .await?;
        }
        Commands::AnalyzeZoneMesh { ffxi_path } => {
            analyze_zone_meshes(PathBuf::from(ffxi_path)).await?;
        }
        Commands::MakeDats {
            project_dir,
            yaml_files,
            out,
        } => {
            make_dats(project_dir, &yaml_files, out)?;
        }
        Commands::ScanDats { ffxi_path } => {
            scan_dats(ffxi_path)?;
        }
        Commands::AuditDats {
            ffxi_path,
            out,
            xbox_packages,
        } => {
            audit_dats(ffxi_path, out, xbox_packages)?;
        }
        Commands::ExportClientGlobals { runtime_root, out } => {
            export_client_globals(runtime_root, out)?;
        }
        Commands::ExportItems { runtime_root, out } => {
            export_items(runtime_root, out)?;
        }
        Commands::ExportZoneText { runtime_root, out } => {
            export_zone_text(runtime_root, out)?;
        }
        Commands::ExportZoneEntities { runtime_root, out } => {
            export_zone_entities(runtime_root, out)?;
        }
        Commands::ExportDat {
            ffxi_path,
            dat_path,
            dat_id,
            out_path,
        } => {
            export_dat(ffxi_path, dat_path, dat_id.map(DatId::from), out_path)?;
        }
    }

    Ok(())
}
