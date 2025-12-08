// ABOUTME: Sendspin server with TUI dashboard
// ABOUTME: Interactive terminal UI showing real-time server stats and connected clients

use clap::Parser;
use sendspin::server::{SendspinServer, ServerConfig, ServerStats, TestToneSource, TuiApp};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "sendspin-server-tui")]
#[command(author, version, about = "Sendspin streaming audio server with TUI", long_about = None)]
struct Args {
    /// Address to bind the server to
    #[arg(short, long, default_value = "0.0.0.0:8927")]
    bind: SocketAddr,

    /// Server name
    #[arg(short, long, default_value = "Sendspin Server")]
    name: String,

    /// WebSocket endpoint path
    #[arg(long, default_value = "/sendspin")]
    path: String,

    /// Test tone frequency in Hz (0 for silence)
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
    let args = Args::parse();

    // Create audio source
    let source = Box::new(TestToneSource::new(args.frequency.max(0.0), args.sample_rate));

    // Create server configuration
    let config = ServerConfig::new(&args.name)
        .bind_addr(args.bind)
        .ws_path(args.path)
        .chunk_interval_ms(args.chunk_ms)
        .buffer_ahead_ms(args.buffer_ahead_ms);

    // Create server (takes ownership of config)
    let server = SendspinServer::with_config(config.clone()).with_source(source);

    let config = Arc::new(config);
    let client_manager = server.client_manager();

    // Create stats tracker
    let stats = Arc::new(parking_lot::Mutex::new(ServerStats::new(
        args.sample_rate,
        args.chunk_ms,
    )));

    // Spawn stats updater task (simulates audio chunk tracking)
    let stats_clone = Arc::clone(&stats);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(20));
        loop {
            interval.tick().await;
            let mut s = stats_clone.lock();
            s.chunks_sent += 1;
            s.bytes_sent += 5769; // Typical PCM chunk size
        }
    });

    // Setup TUI terminal
    let mut terminal = sendspin::server::tui::setup_terminal()?;

    // Create TUI app
    let mut tui_app = TuiApp::new(Arc::clone(&config), client_manager, Arc::clone(&stats));

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        server.run().await
    });

    // Run TUI in foreground
    let tui_result = tui_app.run(&mut terminal);

    // Cleanup terminal
    sendspin::server::tui::restore_terminal(&mut terminal)?;

    // Show any TUI errors
    if let Err(err) = tui_result {
        eprintln!("TUI error: {}", err);
    }

    // Cancel server task
    server_handle.abort();

    println!("Server stopped");
    Ok(())
}
