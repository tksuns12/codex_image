use std::io;
use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};
use reqwest::Client;
use serde::Serialize;

use crate::auth::{
    get_access_token_or_error, login_oauth_callback, status_for_cli, AuthStore, OAuthLoginPolicy,
};
use crate::config::{AuthConfig, GenerateConfig};
use crate::diagnostics::CliError;
use crate::openai::{generate_image, ImageGenerationRequest, GPT_IMAGE_MODEL};
use crate::output::write_generation_output;

#[derive(Debug, Parser)]
#[command(name = "codex-image", version, about = "Codex Image CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Login via OpenAI OAuth callback flow.
    Login,
    /// Print machine-readable auth status.
    Status {
        /// Required stable status contract output.
        #[arg(long, required = true, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Generate image artifacts and a manifest for the provided prompt.
    Generate {
        /// Prompt text sent to the stable gpt-image-2 generation contract.
        prompt: String,
        /// Output directory where generated image files and manifest.json are written.
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
    /// Clear local codex-image auth state.
    Logout,
}

#[derive(Debug, Serialize)]
struct LogoutResponse {
    logged_out: bool,
    status: &'static str,
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
        Commands::Login => login().await,
        Commands::Status { json } => status(json),
        Commands::Generate { prompt, out } => generate(prompt, out).await,
        Commands::Logout => logout(),
    }
}

async fn login() -> Result<(), CliError> {
    let config = AuthConfig::from_env()?;
    let auth_store = AuthStore::from_config(&config)?;
    let http_client = Client::new();
    let login_policy = OAuthLoginPolicy::production();

    let auth = login_oauth_callback(&config, &http_client, &login_policy, io::stdout()).await?;
    auth_store.save(&auth)?;

    println!("Login successful.");
    Ok(())
}

fn status(_json: bool) -> Result<(), CliError> {
    let config = AuthConfig::from_env_for_store()?;
    let auth_store = AuthStore::from_config(&config)?;
    let status = status_for_cli(&auth_store)?;

    let line =
        serde_json::to_string(&status).unwrap_or_else(|_| "{\"status\":\"invalid\"}".to_string());
    println!("{line}");

    Ok(())
}

async fn generate(prompt: String, out: PathBuf) -> Result<(), CliError> {
    let auth_config = AuthConfig::from_env_for_store()?;
    let auth_store = AuthStore::from_config(&auth_config)?;
    let access_token = get_access_token_or_error(&auth_store)?;

    let generate_config = GenerateConfig::from_env()?;
    let client = Client::new();

    let request = ImageGenerationRequest {
        prompt: prompt.clone(),
        size: None,
        quality: None,
        background: None,
        output_format: None,
    };

    let response = generate_image(
        &client,
        &generate_config.api_base_url,
        &access_token,
        &request,
    )
    .await?;

    let manifest = write_generation_output(&prompt, GPT_IMAGE_MODEL, &out, &response)?;
    let line = serde_json::to_string(&manifest).map_err(|_| CliError::OutputWriteFailed)?;
    println!("{line}");

    Ok(())
}

fn logout() -> Result<(), CliError> {
    let config = AuthConfig::from_env_for_store()?;
    let auth_store = AuthStore::from_config(&config)?;
    auth_store.clear()?;

    let response = LogoutResponse {
        logged_out: true,
        status: "not_logged_in",
    };

    let line = serde_json::to_string(&response)
        .unwrap_or_else(|_| "{\"logged_out\":true,\"status\":\"not_logged_in\"}".to_string());
    println!("{line}");

    Ok(())
}
