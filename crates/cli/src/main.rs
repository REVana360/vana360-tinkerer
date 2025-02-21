mod export_ximesh;
mod make_dats;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use export_ximesh::export_zone_meshes;
use make_dats::make_dats;

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
    }

    Ok(())
}
