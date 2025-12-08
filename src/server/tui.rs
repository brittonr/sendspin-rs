// ABOUTME: Terminal UI for Sendspin server
// ABOUTME: Real-time dashboard showing server stats, clients, and audio metrics

use crate::server::client_manager::ClientManager;
use crate::server::config::ServerConfig;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Server statistics
pub struct ServerStats {
    /// Server start time
    pub start_time: Instant,
    /// Total audio chunks sent
    pub chunks_sent: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Current sample rate
    pub sample_rate: u32,
    /// Current chunk size
    pub chunk_size_ms: u64,
}

impl ServerStats {
    pub fn new(sample_rate: u32, chunk_size_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            chunks_sent: 0,
            bytes_sent: 0,
            sample_rate,
            chunk_size_ms,
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn chunks_per_second(&self) -> f64 {
        let uptime_secs = self.uptime().as_secs_f64();
        if uptime_secs > 0.0 {
            self.chunks_sent as f64 / uptime_secs
        } else {
            0.0
        }
    }

    pub fn bytes_per_second(&self) -> f64 {
        let uptime_secs = self.uptime().as_secs_f64();
        if uptime_secs > 0.0 {
            self.bytes_sent as f64 / uptime_secs
        } else {
            0.0
        }
    }
}

/// TUI application state
pub struct TuiApp {
    config: Arc<ServerConfig>,
    client_manager: Arc<ClientManager>,
    stats: Arc<parking_lot::Mutex<ServerStats>>,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(
        config: Arc<ServerConfig>,
        client_manager: Arc<ClientManager>,
        stats: Arc<parking_lot::Mutex<ServerStats>>,
    ) -> Self {
        Self {
            config,
            client_manager,
            stats,
            should_quit: false,
        }
    }

    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.should_quit = true;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // Server info
                Constraint::Length(7),  // Stats
                Constraint::Min(10),    // Clients
                Constraint::Length(3),  // Help
            ])
            .split(f.area());

        self.render_server_info(f, chunks[0]);
        self.render_stats(f, chunks[1]);
        self.render_clients(f, chunks[2]);
        self.render_help(f, chunks[3]);
    }

    fn render_server_info(&self, f: &mut Frame, area: Rect) {
        let stats = self.stats.lock();
        let uptime = stats.uptime();
        let uptime_str = format!(
            "{:02}:{:02}:{:02}",
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60,
            uptime.as_secs() % 60
        );

        let text = vec![
            Line::from(vec![
                Span::styled("Server: ", Style::default().fg(Color::Cyan)),
                Span::raw(&self.config.name),
            ]),
            Line::from(vec![
                Span::styled("Endpoint: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "ws://{}{}",
                    self.config.bind_addr, self.config.ws_path
                )),
            ]),
            Line::from(vec![
                Span::styled("Audio: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!(
                    "{}Hz {}ch {}bit PCM",
                    stats.sample_rate,
                    self.config.default_channels,
                    self.config.default_bit_depth
                )),
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(Color::Cyan)),
                Span::raw(uptime_str),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .title("Sendspin Server")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

        f.render_widget(paragraph, area);
    }

    fn render_stats(&self, f: &mut Frame, area: Rect) {
        let stats = self.stats.lock();

        let chunks_per_sec = stats.chunks_per_second();
        let bytes_per_sec = stats.bytes_per_second();
        let mb_per_sec = bytes_per_sec / 1_048_576.0;

        let text = vec![
            Line::from(vec![
                Span::styled("Chunks Sent: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{} ({:.1}/s)", stats.chunks_sent, chunks_per_sec)),
            ]),
            Line::from(vec![
                Span::styled("Data Sent: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    "{:.2} MB ({:.2} MB/s)",
                    stats.bytes_sent as f64 / 1_048_576.0,
                    mb_per_sec
                )),
            ]),
            Line::from(vec![
                Span::styled("Chunk Interval: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{}ms", stats.chunk_size_ms)),
            ]),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .title("Statistics")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

        f.render_widget(paragraph, area);
    }

    fn render_clients(&self, f: &mut Frame, area: Rect) {
        let client_count = self.client_manager.client_count();

        // Collect client data into owned strings first
        struct ClientDisplay {
            name: String,
            client_id: String,
            roles: String,
            format_str: String,
            volume_str: String,
        }

        let mut client_data = Vec::new();

        self.client_manager.for_each(|client| {
            let roles = client.active_roles.join(", ");
            let volume_str = if client.muted {
                format!("{}% (muted)", client.volume)
            } else {
                format!("{}%", client.volume)
            };

            let format_str = if let Some(ref fmt) = client.audio_format {
                format!(
                    "{}Hz {}ch {}bit {}",
                    fmt.sample_rate,
                    fmt.channels,
                    fmt.bit_depth,
                    match fmt.codec {
                        crate::audio::types::Codec::Pcm => "PCM",
                        crate::audio::types::Codec::Opus => "Opus",
                        crate::audio::types::Codec::Flac => "FLAC",
                        crate::audio::types::Codec::Mp3 => "MP3",
                    }
                )
            } else {
                "No format".to_string()
            };

            client_data.push(ClientDisplay {
                name: client.name.clone(),
                client_id: client.client_id.clone(),
                roles,
                format_str,
                volume_str,
            });
        });

        // Now build the list items from owned data
        let mut items = Vec::new();

        for client in &client_data {
            items.push(ListItem::new(vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Magenta)),
                    Span::raw(&client.name),
                ]),
                Line::from(vec![
                    Span::styled("  ID: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&client.client_id),
                ]),
                Line::from(vec![
                    Span::styled("  Roles: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&client.roles),
                ]),
                Line::from(vec![
                    Span::styled("  Format: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&client.format_str),
                ]),
                Line::from(vec![
                    Span::styled("  Volume: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(&client.volume_str),
                ]),
                Line::from(""),
            ]));
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "No clients connected",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ))));
        }

        let list = List::new(items).block(
            Block::default()
                .title(format!("Connected Clients ({})", client_count))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

        f.render_widget(list, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let text = Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(" or ", Style::default().fg(Color::DarkGray)),
            Span::styled("ESC", Style::default().fg(Color::Yellow)),
            Span::styled(" to quit", Style::default().fg(Color::DarkGray)),
        ]);

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        f.render_widget(paragraph, area);
    }
}

/// Setup TUI terminal
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore terminal to normal mode
pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
