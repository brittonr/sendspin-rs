// ABOUTME: Audio encoders for different codecs
// ABOUTME: PCM 24-bit, Opus, and FLAC encoding

use crate::audio::types::{Codec, Sample};

/// Trait for audio encoders
pub trait AudioEncoder: Send + Sync {
    /// Encode samples to bytes
    fn encode(&mut self, samples: &[Sample]) -> Vec<u8>;

    /// Get the codec type
    fn codec(&self) -> Codec;

    /// Get the sample rate this encoder expects
    fn sample_rate(&self) -> u32;

    /// Get the number of channels
    fn channels(&self) -> u8;

    /// Get the bit depth
    fn bit_depth(&self) -> u8;

    /// Get codec header (if any, base64 encoded)
    fn codec_header(&self) -> Option<Vec<u8>> {
        None
    }
}

/// PCM 24-bit little-endian encoder
pub struct PcmEncoder {
    sample_rate: u32,
    channels: u8,
}

impl PcmEncoder {
    /// Create a new PCM encoder
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }
}

impl AudioEncoder for PcmEncoder {
    fn encode(&mut self, samples: &[Sample]) -> Vec<u8> {
        let mut out = Vec::with_capacity(samples.len() * 3);

        for sample in samples {
            // 24-bit little-endian: [low, mid, high]
            let val = sample.0;
            out.push((val & 0xFF) as u8);
            out.push(((val >> 8) & 0xFF) as u8);
            out.push(((val >> 16) & 0xFF) as u8);
        }

        out
    }

    fn codec(&self) -> Codec {
        Codec::Pcm
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        self.channels
    }

    fn bit_depth(&self) -> u8 {
        24
    }
}

/// Opus encoder (placeholder - requires opus crate)
pub struct OpusEncoder {
    sample_rate: u32,
    channels: u8,
    // encoder: Option<opus::Encoder>,
}

impl OpusEncoder {
    /// Create a new Opus encoder
    ///
    /// Note: Opus requires 48kHz sample rate
    pub fn new(sample_rate: u32, channels: u8) -> Result<Self, String> {
        if sample_rate != 48000 {
            return Err("Opus requires 48kHz sample rate".to_string());
        }

        // TODO: Initialize opus encoder when we add the opus crate
        // let encoder = opus::Encoder::new(sample_rate, channels, opus::Application::Audio)?;

        Ok(Self {
            sample_rate,
            channels,
            // encoder: Some(encoder),
        })
    }
}

impl AudioEncoder for OpusEncoder {
    fn encode(&mut self, samples: &[Sample]) -> Vec<u8> {
        // TODO: Implement actual Opus encoding
        // For now, fall back to PCM
        let mut out = Vec::with_capacity(samples.len() * 3);
        for sample in samples {
            let val = sample.0;
            out.push((val & 0xFF) as u8);
            out.push(((val >> 8) & 0xFF) as u8);
            out.push(((val >> 16) & 0xFF) as u8);
        }
        out
    }

    fn codec(&self) -> Codec {
        Codec::Opus
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        self.channels
    }

    fn bit_depth(&self) -> u8 {
        16 // Opus uses 16-bit internally
    }
}

/// FLAC encoder (placeholder - requires flac crate)
pub struct FlacEncoder {
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    // encoder: Option<flac::Encoder>,
}

impl FlacEncoder {
    /// Create a new FLAC encoder
    pub fn new(sample_rate: u32, channels: u8, bit_depth: u8) -> Self {
        Self {
            sample_rate,
            channels,
            bit_depth,
        }
    }
}

impl AudioEncoder for FlacEncoder {
    fn encode(&mut self, samples: &[Sample]) -> Vec<u8> {
        // TODO: Implement actual FLAC encoding
        // For now, fall back to PCM
        let mut out = Vec::with_capacity(samples.len() * 3);
        for sample in samples {
            let val = sample.0;
            out.push((val & 0xFF) as u8);
            out.push(((val >> 8) & 0xFF) as u8);
            out.push(((val >> 16) & 0xFF) as u8);
        }
        out
    }

    fn codec(&self) -> Codec {
        Codec::Flac
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u8 {
        self.channels
    }

    fn bit_depth(&self) -> u8 {
        self.bit_depth
    }

    fn codec_header(&self) -> Option<Vec<u8>> {
        // TODO: Return FLAC stream info header
        None
    }
}

/// Create an encoder for the given codec
pub fn create_encoder(codec: Codec, sample_rate: u32, channels: u8, bit_depth: u8) -> Box<dyn AudioEncoder> {
    match codec {
        Codec::Pcm => Box::new(PcmEncoder::new(sample_rate, channels)),
        Codec::Opus => {
            match OpusEncoder::new(sample_rate, channels) {
                Ok(enc) => Box::new(enc),
                Err(_) => Box::new(PcmEncoder::new(sample_rate, channels)), // Fallback
            }
        }
        Codec::Flac => Box::new(FlacEncoder::new(sample_rate, channels, bit_depth)),
        Codec::Mp3 => {
            // MP3 encoding not supported, fall back to PCM
            Box::new(PcmEncoder::new(sample_rate, channels))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_encode() {
        let mut encoder = PcmEncoder::new(48000, 2);

        let samples = vec![
            Sample(0x123456),
            Sample(-0x123456),
            Sample(0),
            Sample(Sample::MAX.0),
        ];

        let encoded = encoder.encode(&samples);

        // Each sample should be 3 bytes
        assert_eq!(encoded.len(), 12);

        // First sample: 0x123456 -> [0x56, 0x34, 0x12]
        assert_eq!(encoded[0], 0x56);
        assert_eq!(encoded[1], 0x34);
        assert_eq!(encoded[2], 0x12);
    }

    #[test]
    fn test_encoder_traits() {
        let encoder = PcmEncoder::new(48000, 2);
        assert_eq!(encoder.codec(), Codec::Pcm);
        assert_eq!(encoder.sample_rate(), 48000);
        assert_eq!(encoder.channels(), 2);
        assert_eq!(encoder.bit_depth(), 24);
    }
}
