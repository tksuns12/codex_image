#[tokio::main]
async fn main() {
    std::process::exit(codex_image::cli::run().await);
}
