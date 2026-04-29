use clap::{Parser, Subcommand};

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
            eprintln!("{}", err.redacted_message());
            1
        }
    }
}

async fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Login => {
            let _ = AuthConfig::from_env()?;
            Ok(())
        }
    }
}
