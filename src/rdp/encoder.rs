//! Frame encoder for RDP
//!
//! Provides frame encoding capabilities for sending to RDP clients.

#[cfg(feature = "headless-compositor")]
use crate::compositor::integration::RenderedFrame;
#[cfg(feature = "headless-compositor")]
use crate::compositor::types::PixelFormat;
use anyhow::{Context, Result};
use tracing::{debug, trace};

/// Frame encoder format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderFormat {
    /// Raw uncompressed frames
    Raw,

    /// RLE compression
    Rle,

    /// RemoteFX codec (future)
    RemoteFx,

    /// H.264 codec (future)
    H264,
}

/// Encoded frame ready for transmission
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    /// Frame sequence number
    pub sequence: u64,

    /// Encoded data
    pub data: Vec<u8>,

    /// Encoding format used
    pub format: EncoderFormat,

    /// Original dimensions
    pub width: u32,
    pub height: u32,

    /// Compression ratio (0.0 - 1.0)
    pub compression_ratio: f32,
}

/// Frame encoder
pub struct FrameEncoder {
    /// Encoder format
    format: EncoderFormat,

    /// Target pixel format
    target_format: PixelFormat,
}

impl FrameEncoder {
    /// Create new frame encoder
    pub fn new(format: EncoderFormat) -> Self {
        debug!("Creating frame encoder with format: {:?}", format);

        Self {
            format,
            target_format: PixelFormat::BGRA8888, // RDP typically uses BGRA
        }
    }

    /// Encode a rendered frame
    pub fn encode(&self, frame: &RenderedFrame) -> Result<EncodedFrame> {
        trace!("Encoding frame {} with {:?}", frame.sequence, self.format);

        let (width, height) = frame.dimensions();
        let pixel_data = frame.pixel_data();

        let encoded_data = match self.format {
            EncoderFormat::Raw => self.encode_raw(pixel_data, frame.pixel_format())?,
            EncoderFormat::Rle => self.encode_rle(pixel_data, frame.pixel_format())?,
            EncoderFormat::RemoteFx => {
                // Placeholder for future RemoteFX implementation
                debug!("RemoteFX not yet implemented, falling back to raw");
                self.encode_raw(pixel_data, frame.pixel_format())?
            }
            EncoderFormat::H264 => {
                // Placeholder for future H.264 implementation
                debug!("H.264 not yet implemented, falling back to raw");
                self.encode_raw(pixel_data, frame.pixel_format())?
            }
        };

        let compression_ratio = encoded_data.len() as f32 / pixel_data.len() as f32;

        debug!(
            "Frame {} encoded: {} -> {} bytes (ratio: {:.2})",
            frame.sequence,
            pixel_data.len(),
            encoded_data.len(),
            compression_ratio
        );

        Ok(EncodedFrame {
            sequence: frame.sequence,
            data: encoded_data,
            format: self.format,
            width,
            height,
            compression_ratio,
        })
    }

    /// Encode frame as raw (uncompressed)
    fn encode_raw(&self, data: &[u8], source_format: PixelFormat) -> Result<Vec<u8>> {
        // If formats match, just clone the data
        if source_format == self.target_format {
            return Ok(data.to_vec());
        }

        // Convert pixel format if needed
        self.convert_pixel_format(data, source_format)
    }

    /// Encode frame with RLE compression
    fn encode_rle(&self, data: &[u8], source_format: PixelFormat) -> Result<Vec<u8>> {
        // First convert to target format if needed
        let converted = if source_format == self.target_format {
            data.to_vec()
        } else {
            self.convert_pixel_format(data, source_format)?
        };

        // Simple RLE compression
        let mut compressed = Vec::with_capacity(converted.len());
        let bpp = self.target_format.bytes_per_pixel();

        let mut i = 0;
        while i < converted.len() {
            let pixel = &converted[i..i + bpp];

            // Count consecutive identical pixels
            let mut count = 1u8;
            let mut j = i + bpp;

            while j < converted.len() && count < 255 {
                if &converted[j..j + bpp] == pixel {
                    count += 1;
                    j += bpp;
                } else {
                    break;
                }
            }

            // Write run: count + pixel data
            compressed.push(count);
            compressed.extend_from_slice(pixel);

            i = j;
        }

        Ok(compressed)
    }

    /// Convert pixel format
    fn convert_pixel_format(&self, data: &[u8], source_format: PixelFormat) -> Result<Vec<u8>> {
        let bpp = source_format.bytes_per_pixel();
        let pixel_count = data.len() / bpp;
        let mut converted = Vec::with_capacity(pixel_count * self.target_format.bytes_per_pixel());

        for i in 0..pixel_count {
            let offset = i * bpp;
            let pixel = &data[offset..offset + bpp];

            // Extract RGBA components from source
            let (r, g, b, a) = match source_format {
                PixelFormat::BGRA8888 => (pixel[2], pixel[1], pixel[0], pixel[3]),
                PixelFormat::RGBA8888 => (pixel[0], pixel[1], pixel[2], pixel[3]),
                PixelFormat::BGRX8888 => (pixel[2], pixel[1], pixel[0], 255),
                PixelFormat::RGBX8888 => (pixel[0], pixel[1], pixel[2], 255),
            };

            // Write to target format
            match self.target_format {
                PixelFormat::BGRA8888 => {
                    converted.push(b);
                    converted.push(g);
                    converted.push(r);
                    converted.push(a);
                }
                PixelFormat::RGBA8888 => {
                    converted.push(r);
                    converted.push(g);
                    converted.push(b);
                    converted.push(a);
                }
                PixelFormat::BGRX8888 => {
                    converted.push(b);
                    converted.push(g);
                    converted.push(r);
                    converted.push(255);
                }
                PixelFormat::RGBX8888 => {
                    converted.push(r);
                    converted.push(g);
                    converted.push(b);
                    converted.push(255);
                }
            }
        }

        Ok(converted)
    }

    /// Get encoder format
    pub fn format(&self) -> EncoderFormat {
        self.format
    }

    /// Set encoder format
    pub fn set_format(&mut self, format: EncoderFormat) {
        debug!("Changing encoder format from {:?} to {:?}", self.format, format);
        self.format = format;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        let encoder = FrameEncoder::new(EncoderFormat::Raw);
        assert_eq!(encoder.format(), EncoderFormat::Raw);
    }

    #[test]
    fn test_encoder_format_change() {
        let mut encoder = FrameEncoder::new(EncoderFormat::Raw);
        encoder.set_format(EncoderFormat::Rle);
        assert_eq!(encoder.format(), EncoderFormat::Rle);
    }

    #[test]
    fn test_pixel_format_conversion() {
        let encoder = FrameEncoder::new(EncoderFormat::Raw);

        // Create test pixel data in RGBA format
        let rgba_data = vec![255, 0, 0, 255]; // Red pixel

        // Convert to BGRA
        let bgra_data = encoder.convert_pixel_format(&rgba_data, PixelFormat::RGBA8888).unwrap();

        // Should be Blue, Green, Red, Alpha
        assert_eq!(bgra_data, vec![0, 0, 255, 255]);
    }

    #[test]
    fn test_rle_compression() {
        let encoder = FrameEncoder::new(EncoderFormat::Rle);

        // Create data with consecutive identical pixels (BGRA format)
        let mut data = Vec::new();
        let red_pixel = [0, 0, 255, 255u8]; // BGRA red

        // 10 identical red pixels
        for _ in 0..10 {
            data.extend_from_slice(&red_pixel);
        }

        let compressed = encoder.encode_rle(&data, PixelFormat::BGRA8888).unwrap();

        // RLE should compress: count (1 byte) + pixel (4 bytes) = 5 bytes
        // vs original 40 bytes (10 pixels * 4 bytes)
        assert_eq!(compressed.len(), 5);
        assert_eq!(compressed[0], 10); // Count
        assert_eq!(&compressed[1..5], &red_pixel); // Pixel data
    }
}
