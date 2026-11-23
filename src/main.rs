pub mod api;
pub mod core;
pub mod database;
pub mod dashboard;
pub mod integrations;
pub mod notifications;
pub mod scheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    tracing::info!("Starting GXE-Search-Queue service...");

    // Basic startup log output
    println!("GXE-Search-Queue engine started successfully.");

    Ok(())
}
