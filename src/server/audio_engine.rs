// ABOUTME: Audio engine for generating and broadcasting audio chunks
// ABOUTME: Runs a 20ms interval loop to generate synchronized audio

use crate::audio::types::Sample;
use crate::server::audio_source::AudioSource;
use crate::server::client_manager::ClientManager;
use crate::server::clock::ServerClock;
use crate::server::encoder::PcmEncoder;
use crate::server::encoder::AudioEncoder;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior};

/// Audio chunk type byte for player role (per Sendspin Protocol spec)
/// Spec: Binary message type 4 for player role audio chunks
const AUDIO_CHUNK_TYPE: u8 = 0x04;

/// Audio engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Engine is stopped
    Stopped,
    /// Engine is running and streaming
    Running,
    /// Engine is paused (maintains timing but sends silence)
    Paused,
}

/// Audio engine for generating and broadcasting audio chunks
pub struct AudioEngine {
    /// Audio source
    source: Box<dyn AudioSource>,
    /// Client manager for broadcasting
    client_manager: Arc<ClientManager>,
    /// Server clock for timestamps
    clock: Arc<ServerClock>,
    /// Chunk interval
    chunk_interval: Duration,
    /// Samples per chunk (per channel)
    samples_per_chunk: usize,
    /// Buffer ahead time in microseconds
    buffer_ahead_micros: i64,
    /// Current engine state
    state: EngineState,
    /// Encoder for PCM
    encoder: PcmEncoder,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new(
        source: Box<dyn AudioSource>,
        client_manager: Arc<ClientManager>,
        clock: Arc<ServerClock>,
        chunk_interval_ms: u64,
        buffer_ahead_ms: u64,
    ) -> Self {
        let sample_rate = source.sample_rate();
        let samples_per_chunk = (sample_rate as u64 * chunk_interval_ms / 1000) as usize;

        Self {
            source,
            client_manager,
            clock,
            chunk_interval: Duration::from_millis(chunk_interval_ms),
            samples_per_chunk,
            buffer_ahead_micros: (buffer_ahead_ms * 1000) as i64,
            state: EngineState::Stopped,
            encoder: PcmEncoder::new(sample_rate, 2),
        }
    }

    /// Get the current state
    pub fn state(&self) -> EngineState {
        self.state
    }

    /// Start the engine
    pub fn start(&mut self) {
        self.state = EngineState::Running;
    }

    /// Pause the engine
    pub fn pause(&mut self) {
        self.state = EngineState::Paused;
    }

    /// Stop the engine
    pub fn stop(&mut self) {
        self.state = EngineState::Stopped;
    }

    /// Run the audio engine loop
    ///
    /// This should be spawned as a separate task
    pub async fn run(&mut self, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = interval(self.chunk_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        log::info!(
            "Audio engine started: {}ms chunks, {} samples/chunk, {} buffer ahead",
            self.chunk_interval.as_millis(),
            self.samples_per_chunk,
            self.buffer_ahead_micros / 1000
        );

        self.state = EngineState::Running;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if self.state == EngineState::Stopped {
                        continue;
                    }

                    self.generate_and_broadcast_chunk();
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        log::info!("Audio engine shutting down");
                        break;
                    }
                }
            }
        }

        self.state = EngineState::Stopped;
    }

    /// Generate a single audio chunk and broadcast it
    fn generate_and_broadcast_chunk(&mut self) {
        // Get current time and calculate playback timestamp
        let now = self.clock.now_micros();
        let play_at = now + self.buffer_ahead_micros;

        // Generate audio samples
        let samples = if self.state == EngineState::Paused {
            // Send silence when paused
            vec![Sample::ZERO; self.samples_per_chunk * 2]
        } else {
            // Get samples from source
            match self.source.read_chunk(self.samples_per_chunk) {
                Some(samples) => samples,
                None => {
                    // Source exhausted, send silence
                    vec![Sample::ZERO; self.samples_per_chunk * 2]
                }
            }
        };

        // Encode to PCM
        let encoded = self.encoder.encode(&samples);

        // Build binary message: [type=0x04][timestamp: i64 BE][audio data]
        let mut message = Vec::with_capacity(9 + encoded.len());
        message.push(AUDIO_CHUNK_TYPE);
        message.extend_from_slice(&play_at.to_be_bytes());
        message.extend_from_slice(&encoded);

        // Broadcast to all clients
        self.client_manager.broadcast_audio(&message);
    }

    /// Change the audio source
    pub fn set_source(&mut self, source: Box<dyn AudioSource>) {
        self.source = source;
        let sample_rate = self.source.sample_rate();
        self.samples_per_chunk = (sample_rate as u64 * self.chunk_interval.as_millis() as u64 / 1000) as usize;
        self.encoder = PcmEncoder::new(sample_rate, 2);
    }
}

/// Spawn an audio engine task
pub fn spawn_audio_engine(
    source: Box<dyn AudioSource>,
    client_manager: Arc<ClientManager>,
    clock: Arc<ServerClock>,
    chunk_interval_ms: u64,
    buffer_ahead_ms: u64,
) -> (tokio::task::JoinHandle<()>, watch::Sender<bool>) {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let handle = tokio::spawn(async move {
        let mut engine = AudioEngine::new(
            source,
            client_manager,
            clock,
            chunk_interval_ms,
            buffer_ahead_ms,
        );
        engine.run(shutdown_rx).await;
    });

    (handle, shutdown_tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::audio_source::TestToneSource;

    #[test]
    fn test_engine_creation() {
        let source = Box::new(TestToneSource::new(440.0, 48000));
        let client_manager = Arc::new(ClientManager::new());
        let clock = Arc::new(ServerClock::new());

        let engine = AudioEngine::new(source, client_manager, clock, 20, 500);

        assert_eq!(engine.state(), EngineState::Stopped);
        // 48000 Hz * 20ms = 960 samples
        assert_eq!(engine.samples_per_chunk, 960);
    }
}
