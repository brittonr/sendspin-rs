// ABOUTME: Example Sendspin server
// ABOUTME: Demonstrates running a simple streaming server with test tone

use clap::Parser;
use sendspin::server::{SendspinServer, ServerConfig, TestToneSource};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0:8927")]
    bind: SocketAddr,

    /// Server name
    #[arg(short, long, default_value = "Sendspin Rust Server")]
    name: String,

    /// Test tone frequency in Hz
    #[arg(short, long, default_value = "440.0")]
    frequency: f64,

    /// Sample rate in Hz
    #[arg(short, long, default_value = "48000")]
    sample_rate: u32,

    /// Audio chunk interval in milliseconds
    #[arg(long, default_value = "20")]
    chunk_ms: u64,

    /// Buffer ahead time in milliseconds
    #[arg(long, default_value = "500")]
    buffer_ahead_ms: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sendspin=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    tracing::info!("Starting Sendspin server...");
    tracing::info!("  Bind address: {}", args.bind);
    tracing::info!("  Server name: {}", args.name);
    tracing::info!("  Test tone: {} Hz", args.frequency);
    tracing::info!("  Sample rate: {} Hz", args.sample_rate);
    tracing::info!("  Chunk interval: {} ms", args.chunk_ms);
    tracing::info!("  Buffer ahead: {} ms", args.buffer_ahead_ms);

    // Create audio source (test tone)
    let source = Box::new(TestToneSource::new(args.frequency, args.sample_rate));

    // Create server configuration
    let config = ServerConfig::new(&args.name)
        .bind_addr(args.bind)
        .chunk_interval_ms(args.chunk_ms)
        .buffer_ahead_ms(args.buffer_ahead_ms);

    // Create and run server
    let server = SendspinServer::with_config(config).with_source(source);

    tracing::info!("Server ready. Connect with a Sendspin client to ws://{}/sendspin", args.bind);

    server.run().await
}
