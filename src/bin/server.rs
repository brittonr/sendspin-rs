// ABOUTME: Sendspin server binary
// ABOUTME: Standalone server application for streaming audio

use clap::Parser;
use sendspin::server::{SendspinServer, ServerArgs};

#[derive(Parser, Debug)]
#[command(name = "sendspin-server")]
#[command(author, version, about = "Sendspin streaming audio server", long_about = None)]
struct Args {
    #[command(flatten)]
    server: ServerArgs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    // Initialize tracing
    args.server.init_tracing();

    // Log startup info
    args.server.log_startup_info();

    // Create audio source
    let source = args.server.create_audio_source()?;

    // Create server configuration
    let config = args.server.build_config();

    // Create and run server
    let server = SendspinServer::with_config(config).with_source(source);
    let client_manager = server.client_manager();

    // Spawn a task to periodically report connected clients
    let report_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let count = client_manager.client_count();
            if count > 0 {
                tracing::info!("Connected clients: {}", count);
                client_manager.for_each(|client| {
                    tracing::info!(
                        "  - {} ({}): roles={:?}, volume={}%, muted={}",
                        client.name,
                        client.client_id,
                        client.active_roles,
                        client.volume,
                        client.muted
                    );
                });
            }
        }
    });

    tracing::info!("Press Ctrl+C to stop");

    let result = server.run().await;
    report_task.abort();
    result
}
