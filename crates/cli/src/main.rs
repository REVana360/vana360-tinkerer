mod export_dat;
mod export_ximesh;
mod make_dats;
mod scan_dats;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
