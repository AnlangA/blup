use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use content_pipeline::ContentPipeline;

#[derive(Parser)]
#[command(
    name = "content-importer",
    version = "0.1.0",
    about = "Import learning materials from various formats"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import a PDF file
    ImportPdf {
        /// Path to PDF file
        #[arg(value_name = "PDF_FILE")]
        pdf_file: PathBuf,

        /// Output JSON path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Maximum chunk size in characters
        #[arg(long, default_value = "4000")]
        max_chunk_size: usize,

        /// Chunk overlap in characters
        #[arg(long, default_value = "200")]
        chunk_overlap: usize,
    },

    /// Import a Markdown file
    ImportMarkdown {
        /// Path to Markdown file
        #[arg(value_name = "MD_FILE")]
        md_file: PathBuf,

        /// Output JSON path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Import a plain text file
    ImportText {
        /// Path to text file
        #[arg(value_name = "TEXT_FILE")]
        text_file: PathBuf,

        /// Output JSON path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Import from a website URL
    ImportWebsite {
        /// Website URL
        #[arg(value_name = "URL")]
        url: String,

        /// Output JSON path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Request timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u32,
    },

    /// Detect file type and import
    Import {
        /// Path to file or URL
        #[arg(value_name = "SOURCE")]
        source: String,

        /// Output JSON path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let pipeline = ContentPipeline::new();

    match cli.command {
        Commands::ImportPdf {
            pdf_file,
            output,
            max_chunk_size: _,
            chunk_overlap: _,
        } => {
            println!("Importing PDF: {:?}", pdf_file);
            let doc = pipeline.import_file(&pdf_file).await?;
            print_document(&doc, output)?;
        }
        Commands::ImportMarkdown { md_file, output } => {
            println!("Importing Markdown: {:?}", md_file);
            let doc = pipeline.import_file(&md_file).await?;
            print_document(&doc, output)?;
        }
        Commands::ImportText { text_file, output } => {
            println!("Importing text: {:?}", text_file);
            let doc = pipeline.import_file(&text_file).await?;
            print_document(&doc, output)?;
        }
        Commands::ImportWebsite {
            url,
            output,
            timeout: _,
        } => {
            println!("Importing website: {}", url);
            let doc = pipeline.import_website(&url).await?;
            print_document(&doc, output)?;
        }
        Commands::Import { source, output } => {
            // Check if source is a URL or file path
            if source.starts_with("http://") || source.starts_with("https://") {
                println!("Importing website: {}", source);
                let doc = pipeline.import_website(&source).await?;
                print_document(&doc, output)?;
            } else {
                let path = PathBuf::from(&source);
                println!("Importing file: {:?}", path);
                let doc = pipeline.import_file(&path).await?;
                print_document(&doc, output)?;
            }
        }
    }

    Ok(())
}

fn print_document(
    doc: &content_pipeline::models::SourceDocument,
    output: Option<PathBuf>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(doc)?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json)?;
            println!("Document saved to: {:?}", path);
        }
        None => {
            println!("{}", json);
        }
    }

    println!("\nSummary:");
    println!("  Title: {}", doc.title);
    println!("  Type: {}", doc.source_type);
    println!("  Words: {}", doc.metadata.word_count);
    println!("  Chunks: {}", doc.chunks.len());
    println!(
        "  Language: {}",
        doc.language.as_deref().unwrap_or("unknown")
    );
    println!("  Checksum: {}", doc.checksum);

    Ok(())
}
