use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod builder;
mod config;
mod error;

use builder::SandboxBuilder;
use config::BuildConfig;

#[derive(Parser)]
#[command(name = "sandbox-builder")]
#[command(about = "Build sandbox Docker images reproducibly")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, default_value = "sandboxes/definitions")]
    definitions_dir: PathBuf,

    #[arg(long, default_value = "sandboxes/docker")]
    dockerfiles_dir: PathBuf,

    #[arg(long, default_value = "latest")]
    tag: String,

    #[arg(long)]
    no_cache: bool,

    #[arg(long)]
    push: bool,

    #[arg(long)]
    registry: Option<String>,

    #[arg(long, default_value = "linux/amd64")]
    platform: String,

    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a specific sandbox image
    Build {
        /// Sandbox to build (python, node, rust, typst, math, all)
        sandbox: String,
    },
    /// Scan a built image for vulnerabilities
    Scan {
        /// Image to scan
        image: String,
    },
    /// Verify image digest matches expected value
    Verify {
        /// Image to verify
        image: String,
    },
    /// List all sandbox definitions with build status
    List,
    /// Remove built images
    Clean,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let config = BuildConfig {
        definitions_dir: cli.definitions_dir,
        dockerfiles_dir: cli.dockerfiles_dir,
        tag: cli.tag,
        no_cache: cli.no_cache,
        push: cli.push,
        registry: cli.registry,
        platform: cli.platform,
        output: cli.output,
    };

    let builder = SandboxBuilder::new(config);

    match cli.command {
        Commands::Build { sandbox } => {
            if sandbox == "all" {
                builder.build_all().await?;
            } else {
                builder.build(&sandbox).await?;
            }
        }
        Commands::Scan { image } => {
            builder.scan(&image).await?;
        }
        Commands::Verify { image } => {
            builder.verify(&image).await?;
        }
        Commands::List => {
            builder.list().await?;
        }
        Commands::Clean => {
            builder.clean().await?;
        }
    }

    Ok(())
}
