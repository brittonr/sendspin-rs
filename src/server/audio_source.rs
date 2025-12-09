// ABOUTME: Audio source abstraction
// ABOUTME: Provides test tone and file-based audio sources

use crate::audio::types::Sample;
use std::f64::consts::PI;

/// Trait for audio sources
pub trait AudioSource: Send + Sync {
    /// Read the next chunk of audio samples (interleaved stereo)
    /// Returns None when the source is exhausted
    fn read_chunk(&mut self, samples_per_channel: usize) -> Option<Vec<Sample>>;

    /// Get the sample rate in Hz
    fn sample_rate(&self) -> u32;

    /// Get the number of channels
    fn channels(&self) -> u8;

    /// Check if the source is exhausted
    fn is_exhausted(&self) -> bool;

    /// Reset the source to the beginning (if supported)
    fn reset(&mut self) {}
}

/// Test tone source (generates a sine wave)
pub struct TestToneSource {
    frequency: f64,
    sample_rate: u32,
    phase: f64,
    amplitude: f64,
}

impl TestToneSource {
    /// Create a new test tone source
    ///
    /// # Arguments
    /// * `frequency` - Tone frequency in Hz (e.g., 440.0 for A4)
    /// * `sample_rate` - Sample rate in Hz (e.g., 48000)
    pub fn new(frequency: f64, sample_rate: u32) -> Self {
        Self {
            frequency,
            sample_rate,
            phase: 0.0,
            // Use 50% amplitude to avoid clipping
            amplitude: 0.5 * Sample::MAX.0 as f64,
        }
    }

    /// Set the amplitude (0.0 to 1.0)
    pub fn with_amplitude(mut self, amplitude: f64) -> Self {
        self.amplitude = amplitude.clamp(0.0, 1.0) * Sample::MAX.0 as f64;
        self
    }
}

impl AudioSource for TestToneSource {
    fn read_chunk(&mut self, samples_per_channel: usize) -> Option<Vec<Sample>> {
        let mut samples = Vec::with_capacity(samples_per_channel * 2); // stereo

        let phase_increment = 2.0 * PI * self.frequency / self.sample_rate as f64;

        for _ in 0..samples_per_channel {
            let value = (self.phase.sin() * self.amplitude) as i32;
            let sample = Sample(value);

            // Interleaved stereo: L, R, L, R, ...
            samples.push(sample);
            samples.push(sample);

            self.phase += phase_increment;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }

        Some(samples)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        2 // Always stereo
    }

    fn is_exhausted(&self) -> bool {
        false // Test tone never exhausts
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// Silence source (generates silence)
pub struct SilenceSource {
    sample_rate: u32,
}

impl SilenceSource {
    /// Create a new silence source
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }
}

impl AudioSource for SilenceSource {
    fn read_chunk(&mut self, samples_per_channel: usize) -> Option<Vec<Sample>> {
        Some(vec![Sample::ZERO; samples_per_channel * 2])
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        2
    }

    fn is_exhausted(&self) -> bool {
        false
    }
}

/// File-based audio source using symphonia for decoding
pub struct FileSource {
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    format: Box<dyn symphonia::core::formats::FormatReader>,
    track_id: u32,
    sample_rate: u32,
    channels: u8,
    sample_buf: symphonia::core::audio::SampleBuffer<i32>,
    buffer_pos: usize,
    exhausted: bool,
    loop_playback: bool,
}

impl FileSource {
    /// Create a new file source from an audio file path
    ///
    /// Supports: MP3, FLAC, WAV, AAC, and other formats via symphonia
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        // Open the media source
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a probe hint using the file extension
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension() {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

        let format = probed.format;

        // Find the first audio track (skip video/image tracks like album art)
        // Audio tracks will have sample_rate set, video/image tracks won't
        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL
                    && t.codec_params.sample_rate.is_some()
            })
            .ok_or("No audio track found")?;

        let track_id = track.id;

        // Get audio parameters
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.ok_or("Sample rate not found")? as u32;
        let channel_layout = codec_params.channels.ok_or("Channel count not found")?;
        let channels = channel_layout.count() as u8;

        // Create a decoder for the track
        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())?;

        // Create a sample buffer for decoded audio
        // We'll allocate it with a reasonable initial size and resize as needed
        let capacity = 48000 * channels as usize; // 1 second of audio
        let spec = symphonia::core::audio::SignalSpec::new(sample_rate, channel_layout);
        let sample_buf = symphonia::core::audio::SampleBuffer::new(capacity as u64, spec);

        Ok(Self {
            decoder,
            format,
            track_id,
            sample_rate,
            channels,
            sample_buf,
            buffer_pos: 0,
            exhausted: false,
            loop_playback: true, // Loop by default
        })
    }

    /// Set whether to loop playback (default: true)
    pub fn with_loop(mut self, loop_playback: bool) -> Self {
        self.loop_playback = loop_playback;
        self
    }

    fn decode_next_packet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use symphonia::core::errors::Error;

        loop {
            // Get the next packet from the format reader
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    // Decoder needs to be reset
                    self.decoder.reset();
                    continue;
                }
                Err(Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if self.loop_playback {
                        // Reset to beginning
                        self.format.seek(symphonia::core::formats::SeekMode::Accurate,
                                       symphonia::core::formats::SeekTo::TimeStamp { ts: 0, track_id: self.track_id })?;
                        self.decoder.reset();
                        continue;
                    } else {
                        self.exhausted = true;
                        return Err("End of stream".into());
                    }
                }
                Err(e) => return Err(e.into()),
            };

            // Skip packets for other tracks
            if packet.track_id() != self.track_id {
                continue;
            }

            // Decode the packet into audio samples
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    // Copy decoded samples into our sample buffer
                    self.sample_buf.copy_interleaved_ref(decoded);
                    self.buffer_pos = 0;
                    return Ok(());
                }
                Err(Error::DecodeError(err)) => {
                    log::warn!("Decode error: {}", err);
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl AudioSource for FileSource {
    fn read_chunk(&mut self, samples_per_channel: usize) -> Option<Vec<Sample>> {
        if self.exhausted {
            return None;
        }

        let mut output = Vec::with_capacity(samples_per_channel * 2); // stereo

        while output.len() < samples_per_channel * 2 {
            // If we've consumed all samples from the current buffer, decode more
            if self.buffer_pos >= self.sample_buf.len() {
                if self.decode_next_packet().is_err() {
                    // End of file or error
                    if output.is_empty() {
                        return None;
                    } else {
                        // Pad with silence
                        while output.len() < samples_per_channel * 2 {
                            output.push(Sample::ZERO);
                        }
                        break;
                    }
                }
            }

            let samples = self.sample_buf.samples();
            let remaining = samples.len() - self.buffer_pos;
            let needed = (samples_per_channel * 2) - output.len();
            let to_copy = remaining.min(needed);

            // Convert samples based on channel count
            match self.channels {
                1 => {
                    // Mono: duplicate to stereo
                    for i in 0..to_copy {
                        let sample = samples[self.buffer_pos + i];
                        output.push(Sample(sample));
                        output.push(Sample(sample));
                    }
                }
                2 => {
                    // Stereo: direct copy
                    for i in 0..to_copy {
                        output.push(Sample(samples[self.buffer_pos + i]));
                    }
                }
                _ => {
                    // Multi-channel: downmix to stereo (take first 2 channels)
                    let stride = self.channels as usize;
                    for i in (0..to_copy).step_by(stride) {
                        if self.buffer_pos + i + 1 < samples.len() {
                            output.push(Sample(samples[self.buffer_pos + i]));
                            output.push(Sample(samples[self.buffer_pos + i + 1]));
                        }
                    }
                }
            }

            self.buffer_pos += to_copy;
        }

        Some(output)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        2 // Always output stereo
    }

    fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    fn reset(&mut self) {
        use symphonia::core::formats::{SeekMode, SeekTo};

        if let Err(e) = self.format.seek(SeekMode::Accurate, SeekTo::TimeStamp { ts: 0, track_id: self.track_id }) {
            log::warn!("Failed to reset file source: {}", e);
        }
        self.decoder.reset();
        self.buffer_pos = 0;
        self.exhausted = false;
    }
}

/// URL-based audio source for streaming from HTTP/HTTPS
/// Supports MP3, FLAC, WAV, AAC, and other formats via symphonia
pub struct UrlSource {
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    format: Box<dyn symphonia::core::formats::FormatReader>,
    track_id: u32,
    sample_rate: u32,
    channels: u8,
    sample_buf: symphonia::core::audio::SampleBuffer<i32>,
    buffer_pos: usize,
    exhausted: bool,
    url: String,
}

impl UrlSource {
    /// Create a new URL source from an HTTP/HTTPS URL
    ///
    /// Supports: MP3, FLAC, WAV, AAC, and other formats via symphonia
    /// Note: This creates a blocking HTTP request and buffers the response
    pub fn new(url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        log::info!("Opening URL stream: {}", url);

        // Fetch the URL using ureq (pure sync, no runtime conflicts)
        // Note: No timeout for streaming - we want to keep connection open indefinitely
        let response = ureq::get(url)
            .call()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        // Get content type for format hint
        let content_type = response.header("content-type").map(|s| s.to_string());

        log::debug!("Content-Type: {:?}", content_type);

        // Create a hint based on content type or URL extension
        let mut hint = Hint::new();

        // Try content type first
        if let Some(ref ct) = content_type {
            match ct.as_str() {
                "audio/mpeg" | "audio/mp3" => { hint.with_extension("mp3"); }
                "audio/flac" => { hint.with_extension("flac"); }
                "audio/wav" | "audio/x-wav" => { hint.with_extension("wav"); }
                "audio/aac" | "audio/x-aac" => { hint.with_extension("aac"); }
                "audio/ogg" => { hint.with_extension("ogg"); }
                "audio/mp4" | "audio/x-m4a" => { hint.with_extension("m4a"); }
                _ => {
                    // Fall back to URL extension
                    if let Some(ext) = url.split('.').last() {
                        let ext = ext.split('?').next().unwrap_or(ext);
                        hint.with_extension(ext);
                    }
                }
            }
        } else if let Some(ext) = url.split('.').last() {
            // No content type, use URL extension
            let ext = ext.split('?').next().unwrap_or(ext);
            hint.with_extension(ext);
        }

        // Wrap response reader in ReadOnlySource (HTTP streams don't support seeking)
        let reader = response.into_reader();
        let source = ReadOnlySource::new(reader);
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        // Probe the media source to detect format
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

        let format = probed.format;

        // Find the first audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL
                    && t.codec_params.sample_rate.is_some()
            })
            .ok_or("No audio track found in stream")?;

        let track_id = track.id;

        // Get audio parameters
        let codec_params = &track.codec_params;
        let sample_rate = codec_params.sample_rate.ok_or("Sample rate not found")? as u32;
        let channel_layout = codec_params.channels.ok_or("Channel count not found")?;
        let channels = channel_layout.count() as u8;

        log::info!(
            "URL stream opened: {}Hz, {} channels",
            sample_rate,
            channels
        );

        // Create a decoder for the track
        let decoder = symphonia::default::get_codecs()
            .make(codec_params, &DecoderOptions::default())?;

        // Create a sample buffer for decoded audio
        let capacity = sample_rate as usize * channels as usize; // 1 second of audio
        let spec = symphonia::core::audio::SignalSpec::new(sample_rate, channel_layout);
        let sample_buf = symphonia::core::audio::SampleBuffer::new(capacity as u64, spec);

        Ok(Self {
            decoder,
            format,
            track_id,
            sample_rate,
            channels,
            sample_buf,
            buffer_pos: 0,
            exhausted: false,
            url: url.to_string(),
        })
    }

    fn decode_next_packet(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use symphonia::core::errors::Error;

        loop {
            // Get the next packet from the format reader
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // HTTP stream ended (no looping support for streams)
                    self.exhausted = true;
                    return Err("End of stream".into());
                }
                Err(e) => {
                    log::warn!("Error reading from URL stream: {}", e);
                    return Err(e.into());
                }
            };

            // Skip packets for other tracks
            if packet.track_id() != self.track_id {
                continue;
            }

            // Decode the packet into audio samples
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    self.sample_buf.copy_interleaved_ref(decoded);
                    self.buffer_pos = 0;
                    return Ok(());
                }
                Err(Error::DecodeError(err)) => {
                    log::warn!("Decode error in URL stream: {}", err);
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl AudioSource for UrlSource {
    fn read_chunk(&mut self, samples_per_channel: usize) -> Option<Vec<Sample>> {
        if self.exhausted {
            return None;
        }

        let mut output = Vec::with_capacity(samples_per_channel * 2); // stereo

        while output.len() < samples_per_channel * 2 {
            // If we've consumed all samples from the current buffer, decode more
            if self.buffer_pos >= self.sample_buf.len() {
                if self.decode_next_packet().is_err() {
                    // End of stream or error
                    if output.is_empty() {
                        return None;
                    } else {
                        // Pad with silence
                        while output.len() < samples_per_channel * 2 {
                            output.push(Sample::ZERO);
                        }
                        break;
                    }
                }
            }

            let samples = self.sample_buf.samples();
            let remaining = samples.len() - self.buffer_pos;
            let needed = (samples_per_channel * 2) - output.len();
            let to_copy = remaining.min(needed);

            // Convert samples based on channel count (same as FileSource)
            match self.channels {
                1 => {
                    // Mono: duplicate to stereo
                    for i in 0..to_copy {
                        let sample = samples[self.buffer_pos + i];
                        output.push(Sample(sample));
                        output.push(Sample(sample));
                    }
                }
                2 => {
                    // Stereo: direct copy
                    for i in 0..to_copy {
                        output.push(Sample(samples[self.buffer_pos + i]));
                    }
                }
                _ => {
                    // Multi-channel: downmix to stereo (take first 2 channels)
                    let stride = self.channels as usize;
                    for i in (0..to_copy).step_by(stride) {
                        if self.buffer_pos + i + 1 < samples.len() {
                            output.push(Sample(samples[self.buffer_pos + i]));
                            output.push(Sample(samples[self.buffer_pos + i + 1]));
                        }
                    }
                }
            }

            self.buffer_pos += to_copy;
        }

        Some(output)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        2 // Always output stereo
    }

    fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    // Note: reset() is not supported for URL streams (no seeking in HTTP streams)
    // The default no-op implementation is used
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tone_generates_samples() {
        let mut source = TestToneSource::new(440.0, 48000);
        let samples = source.read_chunk(960).unwrap();

        // Should generate stereo samples (960 * 2)
        assert_eq!(samples.len(), 1920);

        // Samples should be within 24-bit range
        for sample in &samples {
            assert!(sample.0 >= Sample::MIN.0);
            assert!(sample.0 <= Sample::MAX.0);
        }
    }

    #[test]
    fn test_tone_never_exhausts() {
        let source = TestToneSource::new(440.0, 48000);
        assert!(!source.is_exhausted());
    }

    #[test]
    fn test_silence_generates_zeros() {
        let mut source = SilenceSource::new(48000);
        let samples = source.read_chunk(960).unwrap();

        assert_eq!(samples.len(), 1920);
        for sample in &samples {
            assert_eq!(sample.0, 0);
        }
    }
}
