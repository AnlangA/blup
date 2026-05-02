use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use content_pipeline::export::TypstRenderer;

#[derive(Parser)]
#[command(
    name = "typst-export",
    version = "0.1.0",
    about = "Export learning content to Typst and PDF"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Export a single chapter to PDF
    ExportChapter {
        /// Path to chapter JSON file
        #[arg(value_name = "CHAPTER_JSON")]
        chapter_json: PathBuf,

        /// Output file path
        #[arg(short, long, default_value = "./output.pdf")]
        output: PathBuf,

        /// Typst template to use
        #[arg(short, long, default_value = "chapter")]
        template: String,

        /// Use Docker sandbox for compilation (default: true)
        #[arg(long)]
        sandbox: bool,

        /// Compile on host (dev only - requires typst CLI)
        #[arg(long)]
        no_sandbox: bool,
    },

    /// Export full curriculum to PDF
    ExportCurriculum {
        /// Path to curriculum JSON file
        #[arg(value_name = "CURRICULUM_JSON")]
        curriculum_json: PathBuf,

        /// Output file path
        #[arg(short, long, default_value = "./curriculum.pdf")]
        output: PathBuf,

        /// Include table of contents
        #[arg(long)]
        toc: bool,

        /// Use Docker sandbox for compilation
        #[arg(long)]
        sandbox: bool,

        /// Compile on host (dev only)
        #[arg(long)]
        no_sandbox: bool,
    },

    /// Render to Typst without compiling
    Render {
        /// Path to input JSON file
        #[arg(value_name = "INPUT_JSON")]
        input_json: PathBuf,

        /// Output format
        #[arg(long, default_value = "typst")]
        format: String,

        /// Template to use
        #[arg(short, long, default_value = "chapter")]
        template: String,
    },

    /// Compile Typst to PDF
    Compile {
        /// Path to Typst file
        #[arg(value_name = "TYPST_FILE")]
        typst_file: PathBuf,

        /// Output PDF path
        #[arg(short, long, default_value = "./output.pdf")]
        output: PathBuf,

        /// Use Docker sandbox
        #[arg(long)]
        sandbox: bool,

        /// Compile on host
        #[arg(long)]
        no_sandbox: bool,
    },

    /// Validate Typst syntax
    Validate {
        /// Path to Typst file
        #[arg(value_name = "TYPST_FILE")]
        typst_file: PathBuf,
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

    match cli.command {
        Commands::ExportChapter {
            chapter_json,
            output,
            template,
            sandbox,
            no_sandbox,
        } => {
            export_chapter(chapter_json, output, template, !no_sandbox).await?;
        }
        Commands::ExportCurriculum {
            curriculum_json,
            output,
            toc,
            sandbox,
            no_sandbox,
        } => {
            export_curriculum(curriculum_json, output, toc, !no_sandbox).await?;
        }
        Commands::Render {
            input_json,
            format,
            template,
        } => {
            render_to_typst(input_json, format, template).await?;
        }
        Commands::Compile {
            typst_file,
            output,
            sandbox,
            no_sandbox,
        } => {
            compile_typst(typst_file, output, !no_sandbox).await?;
        }
        Commands::Validate { typst_file } => {
            validate_typst(typst_file).await?;
        }
    }

    Ok(())
}

async fn export_chapter(
    chapter_json: PathBuf,
    output: PathBuf,
    template: String,
    use_sandbox: bool,
) -> Result<()> {
    println!("Exporting chapter from: {:?}", chapter_json);

    // 1. Read and parse chapter JSON
    let content = std::fs::read_to_string(&chapter_json)?;
    let chapter: serde_json::Value = serde_json::from_str(&content)?;

    // 2. Render to Typst
    let renderer = TypstRenderer::new();
    let typst_source = renderer.render_chapter(&chapter)?;

    if use_sandbox {
        // 3. Compile via sandbox
        println!("Compiling via sandbox...");
        let sandbox =
            sandbox_manager::SandboxManager::new(sandbox_manager::SandboxConfig::default());
        let compiler = content_pipeline::export::TypstCompiler::new(sandbox);
        let artifact = compiler
            .compile_to_pdf(&typst_source, &std::collections::HashMap::new())
            .await?;

        // 4. Write PDF
        std::fs::write(&output, &artifact.data)?;
        println!("PDF exported to: {:?}", output);
        println!(
            "Size: {} bytes, Pages: {:?}",
            artifact.size_bytes, artifact.page_count
        );
    } else {
        // Write Typst source
        let typst_output = output.with_extension("typst");
        std::fs::write(&typst_output, &typst_source)?;
        println!("Typst source written to: {:?}", typst_output);
        println!(
            "Compile with: typst compile {:?} {:?}",
            typst_output, output
        );
    }

    Ok(())
}

async fn export_curriculum(
    curriculum_json: PathBuf,
    output: PathBuf,
    toc: bool,
    use_sandbox: bool,
) -> Result<()> {
    println!("Exporting curriculum from: {:?}", curriculum_json);

    let content = std::fs::read_to_string(&curriculum_json)?;
    let curriculum: serde_json::Value = serde_json::from_str(&content)?;

    let renderer = TypstRenderer::new();
    let typst_source = renderer.render_curriculum(&curriculum)?;

    if use_sandbox {
        println!("Compiling via sandbox...");
        let sandbox =
            sandbox_manager::SandboxManager::new(sandbox_manager::SandboxConfig::default());
        let compiler = content_pipeline::export::TypstCompiler::new(sandbox);
        let artifact = compiler
            .compile_to_pdf(&typst_source, &std::collections::HashMap::new())
            .await?;

        std::fs::write(&output, &artifact.data)?;
        println!("PDF exported to: {:?}", output);
    } else {
        let typst_output = output.with_extension("typst");
        std::fs::write(&typst_output, &typst_source)?;
        println!("Typst source written to: {:?}", typst_output);
    }

    Ok(())
}

async fn render_to_typst(input_json: PathBuf, format: String, template: String) -> Result<()> {
    println!("Rendering to Typst from: {:?}", input_json);

    let content = std::fs::read_to_string(&input_json)?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let renderer = TypstRenderer::new();
    let typst_source = match template.as_str() {
        "curriculum" => renderer.render_curriculum(&data)?,
        _ => renderer.render_chapter(&data)?,
    };

    match format.as_str() {
        "typst" => {
            println!("{}", typst_source);
        }
        _ => {
            println!("{}", typst_source);
        }
    }

    Ok(())
}

async fn compile_typst(typst_file: PathBuf, output: PathBuf, use_sandbox: bool) -> Result<()> {
    println!("Compiling Typst file: {:?}", typst_file);

    let typst_source = std::fs::read_to_string(&typst_file)?;

    if use_sandbox {
        let sandbox =
            sandbox_manager::SandboxManager::new(sandbox_manager::SandboxConfig::default());
        let compiler = content_pipeline::export::TypstCompiler::new(sandbox);
        let artifact = compiler
            .compile_to_pdf(&typst_source, &std::collections::HashMap::new())
            .await?;

        std::fs::write(&output, &artifact.data)?;
        println!("PDF compiled to: {:?}", output);
    } else {
        // Use host typst CLI
        let status = std::process::Command::new("typst")
            .args([
                "compile",
                typst_file.to_str().unwrap(),
                output.to_str().unwrap(),
            ])
            .status()?;

        if status.success() {
            println!("PDF compiled to: {:?}", output);
        } else {
            anyhow::bail!("Typst compilation failed");
        }
    }

    Ok(())
}

async fn validate_typst(typst_file: PathBuf) -> Result<()> {
    println!("Validating Typst file: {:?}", typst_file);

    let typst_source = std::fs::read_to_string(&typst_file)?;

    // Basic validation: check for common syntax issues
    let has_document_start = typst_source.contains('#');
    let has_balanced_brackets = check_balanced_brackets(&typst_source);

    if has_document_start && has_balanced_brackets {
        println!("Validation passed");
    } else {
        println!("Validation warnings:");
        if !has_document_start {
            println!("  - No Typst commands found (missing #)");
        }
        if !has_balanced_brackets {
            println!("  - Unbalanced brackets detected");
        }
    }

    Ok(())
}

fn check_balanced_brackets(text: &str) -> bool {
    let mut stack = Vec::new();

    for c in text.chars() {
        match c {
            '[' | '(' | '{' => stack.push(c),
            ']' => {
                if stack.pop() != Some('[') {
                    return false;
                }
            }
            ')' => {
                if stack.pop() != Some('(') {
                    return false;
                }
            }
            '}' => {
                if stack.pop() != Some('{') {
                    return false;
                }
            }
            _ => {}
        }
    }

    stack.is_empty()
}
