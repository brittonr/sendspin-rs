// ABOUTME: Sendspin server with TUI dashboard
// ABOUTME: Interactive terminal UI showing real-time server stats and connected clients

use clap::Parser;
use sendspin::server::{SendspinServer, ServerArgs, ServerStats, TuiApp};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "sendspin-server-tui")]
#[command(author, version, about = "Sendspin streaming audio server with TUI", long_about = None)]
struct Args {
    #[command(flatten)]
    server: ServerArgs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    // Initialize tracing
    args.server.init_tracing();

    // Create audio source
    let source = args.server.create_audio_source()?;

    // Get sample rate from source for stats tracking
    let actual_sample_rate = source.sample_rate();

    // Log startup info (after source creation so sample rate is known)
    args.server.log_startup_info();

    // Create server configuration
    let config = args.server.build_config();

    // Create server (takes ownership of config)
    let server = SendspinServer::with_config(config.clone()).with_source(source);

    let config = Arc::new(config);
    let client_manager = server.client_manager();

    // Create stats tracker (use actual sample rate from audio source)
    let stats = Arc::new(parking_lot::Mutex::new(ServerStats::new(
        actual_sample_rate,
        args.server.chunk_ms,
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
    let server_handle = tokio::spawn(async move { server.run().await });

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
