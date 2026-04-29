use std::io;

use clap::{Parser, Subcommand};
use reqwest::Client;

use crate::auth::{login_device_code, AuthStore, DeviceLoginPollPolicy};
use crate::config::AuthConfig;
use crate::diagnostics::CliError;

#[derive(Debug, Parser)]
#[command(name = "codex-image", version, about = "Codex Image CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Login via device-code OAuth flow.
    Login,
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
    }
}

async fn login() -> Result<(), CliError> {
    let config = AuthConfig::from_env()?;
    let auth_store = AuthStore::from_config(&config)?;
    let http_client = Client::new();
    let poll_policy = DeviceLoginPollPolicy::production();

    let auth = login_device_code(&config, &http_client, &poll_policy, io::stdout()).await?;
    auth_store.save(&auth)?;

    println!("Login successful.");
    Ok(())
}
