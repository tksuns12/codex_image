use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::codex::generate_image_with_codex;
use crate::diagnostics::CliError;
use crate::output::write_generation_output_from_files;

const GPT_IMAGE_MODEL: &str = "gpt-image-2";

#[derive(Debug, Parser)]
#[command(name = "codex-image", version, about = "Codex Image CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate image artifacts and a manifest for the provided prompt via installed Codex.
    Generate {
        /// Prompt text passed to Codex's built-in image generation tool.
        prompt: String,
        /// Output directory where generated image files and manifest.json are written.
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
}

pub async fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let _ = err.print();
            return err.exit_code();
        }
    };

    match dispatch(cli).await {
        Ok(()) => 0,
        Err(err) => {
            let envelope = err.error_envelope();
            let line = serde_json::to_string(&envelope).unwrap_or_else(|_| {
                "{\"error\":{\"code\":\"unknown\",\"message\":\"unexpected failure\",\"recoverable\":false,\"hint\":\"Re-run with supported commands or update the binary.\"}}".to_string()
            });
            eprintln!("{line}");
            err.exit_code().as_i32()
        }
    }
}

async fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Generate { prompt, out } => generate(prompt, out).await,
    }
}

async fn generate(prompt: String, out: PathBuf) -> Result<(), CliError> {
    let generated = generate_image_with_codex(&prompt, &out)?;
    let manifest = write_generation_output_from_files(
        &prompt,
        GPT_IMAGE_MODEL,
        &out,
        &[generated.source_path],
    )?;
    let line = serde_json::to_string(&manifest).map_err(|_| CliError::OutputWriteFailed)?;
    println!("{line}");

    Ok(())
}
