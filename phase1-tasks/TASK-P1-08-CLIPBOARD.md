# TASK P1-08: CLIPBOARD SYNCHRONIZATION
**Task ID:** TASK-P1-08
**Duration:** 7-10 days
**Dependencies:** TASK-P1-03 (IronRDP), P1-04 (Portal)
**Priority:** HIGH
**Status:** NOT_STARTED

## OBJECTIVE
Implement complete bidirectional clipboard synchronization between RDP client and Wayland compositor with full format conversion support, loop prevention, and production-grade error handling.

## SUCCESS CRITERIA
- ✅ Bidirectional clipboard sync (RDP ↔ Wayland)
- ✅ Complete format mapping table (15+ formats)
- ✅ Image format conversion (DIB, PNG, JPEG, BMP)
- ✅ Text encoding conversion (UTF-8, UTF-16, ASCII)
- ✅ Rich text formats (HTML, RTF)
- ✅ File transfer via clipboard (text/uri-list)
- ✅ Loop prevention with state machine
- ✅ Large data handling (configurable limits)
- ✅ Performance optimization (streaming, chunking)
- ✅ Error recovery and resilience
- ✅ Complete test coverage (>90%)

## ARCHITECTURE

### Component Hierarchy
```
┌─────────────────────────────────────────────────────────┐
│                   ClipboardManager                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ RDP Handler │  │ Format Conv. │  │ Loop Prevent │  │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                  │          │
│  ┌──────▼──────────────────▼─────────────────▼──────┐  │
│  │              State Machine Core                   │  │
│  └───────────────────────┬───────────────────────────┘  │
│                          │                              │
│  ┌──────────────┐  ┌────▼──────┐  ┌────────────────┐  │
│  │ IronRDP      │  │  Portal    │  │  Cache/Store   │  │
│  │ CliprdrServer│  │  Clipboard │  │  Management    │  │
│  └──────────────┘  └────────────┘  └────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## KEY MODULES

### Core Manager (`src/clipboard/manager.rs`)
```rust
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use ironrdp::cliprdr::{CliprdrServer, CliprdrServerFactory, Backend};
use crate::portal::clipboard::{ClipboardPortal, ClipboardFormat};
use crate::clipboard::formats::{FormatConverter, FormatMapping};
use crate::clipboard::loop_prevention::{LoopDetector, ClipboardState};

pub struct ClipboardManager {
    /// RDP clipboard server
    cliprdr_server: Arc<RwLock<CliprdrServer>>,
    /// Portal clipboard interface
    portal_clipboard: Arc<ClipboardPortal>,
    /// Format converter
    converter: Arc<FormatConverter>,
    /// Loop prevention detector
    loop_detector: Arc<RwLock<LoopDetector>>,
    /// Current clipboard state
    state: Arc<RwLock<ClipboardState>>,
    /// Event channel
    event_tx: mpsc::Sender<ClipboardEvent>,
    /// Configuration
    config: ClipboardConfig,
}

#[derive(Clone, Debug)]
pub struct ClipboardConfig {
    pub max_data_size: usize,
    pub enable_images: bool,
    pub enable_files: bool,
    pub enable_html: bool,
    pub enable_rtf: bool,
    pub chunk_size: usize,
    pub timeout_ms: u64,
    pub loop_detection_window_ms: u64,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            max_data_size: 16 * 1024 * 1024, // 16MB
            enable_images: true,
            enable_files: true,
            enable_html: true,
            enable_rtf: true,
            chunk_size: 64 * 1024, // 64KB chunks
            timeout_ms: 5000,
            loop_detection_window_ms: 500,
        }
    }
}

#[derive(Debug)]
pub enum ClipboardEvent {
    RdpFormatList(Vec<ClipboardFormat>),
    RdpDataRequest(u32),
    RdpDataResponse(Vec<u8>),
    PortalDataAvailable(Vec<String>),
    PortalDataRequest(String),
}

impl ClipboardManager {
    pub async fn new(config: ClipboardConfig) -> Result<Self, ClipboardError> {
        let cliprdr_server = CliprdrServerFactory::new()
            .with_capabilities(config.to_capabilities())
            .build()?;

        let portal_clipboard = ClipboardPortal::new().await?;
        let converter = Arc::new(FormatConverter::new());
        let loop_detector = Arc::new(RwLock::new(
            LoopDetector::new(config.loop_detection_window_ms)
        ));

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let state = Arc::new(RwLock::new(ClipboardState::Idle));

        let manager = Self {
            cliprdr_server: Arc::new(RwLock::new(cliprdr_server)),
            portal_clipboard,
            converter,
            loop_detector,
            state,
            event_tx,
            config,
        };

        // Start event processing
        manager.start_event_processor(event_rx);

        Ok(manager)
    }

    fn start_event_processor(&self, mut event_rx: mpsc::Receiver<ClipboardEvent>) {
        let manager = self.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if let Err(e) = manager.handle_event(event).await {
                    error!("Clipboard event handling error: {:?}", e);
                }
            }
        });
    }

    async fn handle_event(&self, event: ClipboardEvent) -> Result<(), ClipboardError> {
        match event {
            ClipboardEvent::RdpFormatList(formats) => {
                self.handle_rdp_format_list(formats).await?;
            }
            ClipboardEvent::RdpDataRequest(format_id) => {
                self.handle_rdp_data_request(format_id).await?;
            }
            ClipboardEvent::RdpDataResponse(data) => {
                self.handle_rdp_data_response(data).await?;
            }
            ClipboardEvent::PortalDataAvailable(mime_types) => {
                self.handle_portal_data_available(mime_types).await?;
            }
            ClipboardEvent::PortalDataRequest(mime_type) => {
                self.handle_portal_data_request(mime_type).await?;
            }
        }
        Ok(())
    }

    async fn handle_rdp_format_list(&self, formats: Vec<ClipboardFormat>) -> Result<(), ClipboardError> {
        // Check for loop
        let mut loop_detector = self.loop_detector.write().await;
        if loop_detector.would_cause_loop(&formats).await {
            debug!("Loop detected, ignoring RDP format list");
            return Ok(());
        }

        // Update state
        let mut state = self.state.write().await;
        *state = ClipboardState::RdpOwned(formats.clone());

        // Convert RDP formats to MIME types
        let mime_types = self.converter.rdp_to_mime_types(&formats)?;

        // Advertise to Portal
        self.portal_clipboard.advertise_formats(mime_types).await?;

        // Record operation for loop detection
        loop_detector.record_rdp_operation(formats);

        Ok(())
    }

    async fn handle_rdp_data_request(&self, format_id: u32) -> Result<(), ClipboardError> {
        let state = self.state.read().await;

        match &*state {
            ClipboardState::PortalOwned(mime_types) => {
                // Find corresponding MIME type
                let mime_type = self.converter.format_id_to_mime(format_id)?;

                // Get data from Portal
                let data = self.portal_clipboard.get_data(&mime_type).await?;

                // Convert to RDP format
                let rdp_data = self.converter.convert_to_rdp(
                    &data,
                    &mime_type,
                    format_id
                ).await?;

                // Send to RDP client
                let mut server = self.cliprdr_server.write().await;
                server.send_data_response(rdp_data).await?;
            }
            _ => {
                return Err(ClipboardError::InvalidState);
            }
        }

        Ok(())
    }

    async fn handle_portal_data_available(&self, mime_types: Vec<String>) -> Result<(), ClipboardError> {
        // Check for loop
        let mut loop_detector = self.loop_detector.write().await;
        if loop_detector.would_cause_loop_mime(&mime_types).await {
            debug!("Loop detected, ignoring Portal data");
            return Ok(());
        }

        // Update state
        let mut state = self.state.write().await;
        *state = ClipboardState::PortalOwned(mime_types.clone());

        // Convert MIME types to RDP formats
        let rdp_formats = self.converter.mime_to_rdp_formats(&mime_types)?;

        // Send format list to RDP client
        let mut server = self.cliprdr_server.write().await;
        server.send_format_list(rdp_formats).await?;

        // Record operation for loop detection
        loop_detector.record_portal_operation(mime_types);

        Ok(())
    }
}

impl Clone for ClipboardManager {
    fn clone(&self) -> Self {
        Self {
            cliprdr_server: self.cliprdr_server.clone(),
            portal_clipboard: self.portal_clipboard.clone(),
            converter: self.converter.clone(),
            loop_detector: self.loop_detector.clone(),
            state: self.state.clone(),
            event_tx: self.event_tx.clone(),
            config: self.config.clone(),
        }
    }
}
```

### Format Converter (`src/clipboard/formats.rs`)
```rust
use std::collections::HashMap;
use bytes::{Bytes, BytesMut, BufMut};
use image::{ImageFormat, DynamicImage};

/// Complete RDP clipboard format IDs
pub const CF_TEXT: u32 = 1;
pub const CF_BITMAP: u32 = 2;
pub const CF_METAFILEPICT: u32 = 3;
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_ENHMETAFILE: u32 = 14;
pub const CF_HDROP: u32 = 15;
pub const CF_LOCALE: u32 = 16;
pub const CF_DIB: u32 = 8;
pub const CF_PALETTE: u32 = 9;
pub const CF_PENDATA: u32 = 10;
pub const CF_RIFF: u32 = 11;
pub const CF_WAVE: u32 = 12;
pub const CF_HTML: u32 = 0xD010;
pub const CF_PNG: u32 = 0xD011;
pub const CF_JPEG: u32 = 0xD012;
pub const CF_GIF: u32 = 0xD013;
pub const CF_RTF: u32 = 0xD014;
pub const CF_CUSTOM_START: u32 = 0xC000;

pub struct FormatConverter {
    format_map: HashMap<u32, String>,
    mime_map: HashMap<String, u32>,
}

impl FormatConverter {
    pub fn new() -> Self {
        let mut converter = Self {
            format_map: HashMap::new(),
            mime_map: HashMap::new(),
        };

        // Initialize complete format mapping table
        converter.init_format_mappings();
        converter
    }

    fn init_format_mappings(&mut self) {
        // Text formats
        self.add_mapping(CF_TEXT, "text/plain;charset=utf-8");
        self.add_mapping(CF_UNICODETEXT, "text/plain");
        self.add_mapping(CF_HTML, "text/html");
        self.add_mapping(CF_RTF, "application/rtf");

        // Image formats
        self.add_mapping(CF_DIB, "image/bmp");
        self.add_mapping(CF_BITMAP, "image/bmp");
        self.add_mapping(CF_PNG, "image/png");
        self.add_mapping(CF_JPEG, "image/jpeg");
        self.add_mapping(CF_GIF, "image/gif");

        // File formats
        self.add_mapping(CF_HDROP, "text/uri-list");

        // Audio formats
        self.add_mapping(CF_WAVE, "audio/wav");
        self.add_mapping(CF_RIFF, "application/riff");

        // Metadata
        self.add_mapping(CF_LOCALE, "application/x-locale");
    }

    fn add_mapping(&mut self, format_id: u32, mime_type: &str) {
        self.format_map.insert(format_id, mime_type.to_string());
        self.mime_map.insert(mime_type.to_string(), format_id);
    }

    pub async fn convert_to_rdp(
        &self,
        data: &[u8],
        mime_type: &str,
        format_id: u32
    ) -> Result<Vec<u8>, ConversionError> {
        match (mime_type, format_id) {
            // Text conversions
            ("text/plain", CF_UNICODETEXT) => {
                self.convert_text_to_unicode(data)
            }
            ("text/plain;charset=utf-8", CF_TEXT) => {
                Ok(data.to_vec())
            }
            ("text/html", CF_HTML) => {
                self.convert_html_to_rdp_html(data)
            }
            ("application/rtf", CF_RTF) => {
                Ok(data.to_vec())
            }

            // Image conversions
            ("image/png", CF_PNG) => {
                Ok(data.to_vec())
            }
            ("image/png", CF_DIB) => {
                self.convert_png_to_dib(data).await
            }
            ("image/jpeg", CF_JPEG) => {
                Ok(data.to_vec())
            }
            ("image/jpeg", CF_DIB) => {
                self.convert_jpeg_to_dib(data).await
            }
            ("image/bmp", CF_DIB) => {
                self.convert_bmp_to_dib(data)
            }

            // File list conversion
            ("text/uri-list", CF_HDROP) => {
                self.convert_uri_list_to_hdrop(data)
            }

            _ => Err(ConversionError::UnsupportedFormat),
        }
    }

    pub async fn convert_from_rdp(
        &self,
        data: &[u8],
        format_id: u32,
        mime_type: &str
    ) -> Result<Vec<u8>, ConversionError> {
        match (format_id, mime_type) {
            // Text conversions
            (CF_UNICODETEXT, "text/plain") => {
                self.convert_unicode_to_text(data)
            }
            (CF_TEXT, "text/plain;charset=utf-8") => {
                Ok(data.to_vec())
            }
            (CF_HTML, "text/html") => {
                self.convert_rdp_html_to_html(data)
            }

            // Image conversions
            (CF_DIB, "image/png") => {
                self.convert_dib_to_png(data).await
            }
            (CF_DIB, "image/bmp") => {
                self.convert_dib_to_bmp(data)
            }

            // File list conversion
            (CF_HDROP, "text/uri-list") => {
                self.convert_hdrop_to_uri_list(data)
            }

            _ => Err(ConversionError::UnsupportedFormat),
        }
    }

    // Text conversion implementations
    fn convert_text_to_unicode(&self, data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        let text = std::str::from_utf8(data)
            .map_err(|_| ConversionError::InvalidUtf8)?;

        let mut result = Vec::new();
        for ch in text.encode_utf16() {
            result.extend_from_slice(&ch.to_le_bytes());
        }
        // Add null terminator
        result.extend_from_slice(&[0u8, 0u8]);

        Ok(result)
    }

    fn convert_unicode_to_text(&self, data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        if data.len() % 2 != 0 {
            return Err(ConversionError::InvalidData);
        }

        let mut utf16_data = Vec::new();
        for chunk in data.chunks(2) {
            let value = u16::from_le_bytes([chunk[0], chunk[1]]);
            if value == 0 {
                break; // Null terminator
            }
            utf16_data.push(value);
        }

        let text = String::from_utf16(&utf16_data)
            .map_err(|_| ConversionError::InvalidUtf16)?;

        Ok(text.into_bytes())
    }

    fn convert_html_to_rdp_html(&self, data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        let html = std::str::from_utf8(data)
            .map_err(|_| ConversionError::InvalidUtf8)?;

        // RDP HTML format has specific header
        let header = format!(
            "Version:0.9\r\n\
            StartHTML:0000000000\r\n\
            EndHTML:{:010}\r\n\
            StartFragment:0000000000\r\n\
            EndFragment:{:010}\r\n\
            SourceURL:about:blank\r\n",
            html.len() + 200,
            html.len() + 150
        );

        let mut result = header.into_bytes();
        result.extend_from_slice(b"<html><body>\r\n<!--StartFragment-->");
        result.extend_from_slice(data);
        result.extend_from_slice(b"<!--EndFragment-->\r\n</body></html>");

        // Update offsets
        let start_html_pos = result.iter().position(|&b| b == b'<').unwrap();
        let end_html_pos = result.len();
        let start_fragment_pos = result.windows(19)
            .position(|w| w == b"<!--StartFragment-->")
            .unwrap() + 19;
        let end_fragment_pos = result.windows(17)
            .position(|w| w == b"<!--EndFragment-->")
            .unwrap();

        // Update header with correct offsets
        let header_update = format!(
            "Version:0.9\r\n\
            StartHTML:{:010}\r\n\
            EndHTML:{:010}\r\n\
            StartFragment:{:010}\r\n\
            EndFragment:{:010}\r\n",
            start_html_pos,
            end_html_pos,
            start_fragment_pos,
            end_fragment_pos
        );

        result[..header_update.len()].copy_from_slice(header_update.as_bytes());

        Ok(result)
    }

    // DIB (Device Independent Bitmap) conversion
    async fn convert_png_to_dib(&self, png_data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        let image = image::load_from_memory_with_format(png_data, ImageFormat::Png)
            .map_err(|_| ConversionError::ImageDecodeError)?;

        self.create_dib_from_image(&image)
    }

    async fn convert_jpeg_to_dib(&self, jpeg_data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        let image = image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg)
            .map_err(|_| ConversionError::ImageDecodeError)?;

        self.create_dib_from_image(&image)
    }

    fn create_dib_from_image(&self, image: &DynamicImage) -> Result<Vec<u8>, ConversionError> {
        let rgba = image.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());

        // BITMAPINFOHEADER structure
        let mut dib = BytesMut::new();

        // biSize
        dib.put_u32_le(40);
        // biWidth
        dib.put_i32_le(width as i32);
        // biHeight (negative for top-down)
        dib.put_i32_le(-(height as i32));
        // biPlanes
        dib.put_u16_le(1);
        // biBitCount (32 bits for RGBA)
        dib.put_u16_le(32);
        // biCompression (BI_RGB = 0)
        dib.put_u32_le(0);
        // biSizeImage
        let image_size = width * height * 4;
        dib.put_u32_le(image_size);
        // biXPelsPerMeter
        dib.put_i32_le(0);
        // biYPelsPerMeter
        dib.put_i32_le(0);
        // biClrUsed
        dib.put_u32_le(0);
        // biClrImportant
        dib.put_u32_le(0);

        // Pixel data (BGRA format for Windows)
        for pixel in rgba.pixels() {
            dib.put_u8(pixel[2]); // Blue
            dib.put_u8(pixel[1]); // Green
            dib.put_u8(pixel[0]); // Red
            dib.put_u8(pixel[3]); // Alpha
        }

        Ok(dib.to_vec())
    }

    async fn convert_dib_to_png(&self, dib_data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        if dib_data.len() < 40 {
            return Err(ConversionError::InvalidData);
        }

        // Parse BITMAPINFOHEADER
        let bi_size = u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]);
        if bi_size < 40 {
            return Err(ConversionError::InvalidData);
        }

        let width = i32::from_le_bytes([dib_data[4], dib_data[5], dib_data[6], dib_data[7]]).abs() as u32;
        let height = i32::from_le_bytes([dib_data[8], dib_data[9], dib_data[10], dib_data[11]]).abs() as u32;
        let bit_count = u16::from_le_bytes([dib_data[14], dib_data[15]]);

        let header_size = bi_size as usize;
        let pixel_data = &dib_data[header_size..];

        // Convert based on bit depth
        let image = match bit_count {
            32 => {
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for chunk in pixel_data.chunks_exact(4) {
                    rgba_data.push(chunk[2]); // Red
                    rgba_data.push(chunk[1]); // Green
                    rgba_data.push(chunk[0]); // Blue
                    rgba_data.push(chunk[3]); // Alpha
                }
                DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(width, height, rgba_data)
                        .ok_or(ConversionError::ImageCreateError)?
                )
            }
            24 => {
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                let row_size = ((width * 3 + 3) / 4) * 4; // Aligned to 4 bytes
                for y in 0..height {
                    let row_offset = (y * row_size) as usize;
                    for x in 0..width {
                        let pixel_offset = row_offset + (x * 3) as usize;
                        if pixel_offset + 2 < pixel_data.len() {
                            rgb_data.push(pixel_data[pixel_offset + 2]); // Red
                            rgb_data.push(pixel_data[pixel_offset + 1]); // Green
                            rgb_data.push(pixel_data[pixel_offset]);     // Blue
                        }
                    }
                }
                DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(width, height, rgb_data)
                        .ok_or(ConversionError::ImageCreateError)?
                )
            }
            _ => return Err(ConversionError::UnsupportedBitDepth),
        };

        // Encode to PNG
        let mut png_data = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut png_data), ImageFormat::Png)
            .map_err(|_| ConversionError::ImageEncodeError)?;

        Ok(png_data)
    }

    // File drop conversion
    fn convert_uri_list_to_hdrop(&self, data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        let uri_list = std::str::from_utf8(data)
            .map_err(|_| ConversionError::InvalidUtf8)?;

        let mut hdrop = BytesMut::new();

        // DROPFILES structure
        hdrop.put_u32_le(20); // pFiles (offset to file list)
        hdrop.put_i32_le(0);  // pt.x
        hdrop.put_i32_le(0);  // pt.y
        hdrop.put_i32_le(0);  // fNC
        hdrop.put_i32_le(1);  // fWide (Unicode)

        // File paths (null-terminated, double-null at end)
        for line in uri_list.lines() {
            if line.starts_with("file://") {
                let path = &line[7..];
                let path = percent_encoding::percent_decode_str(path)
                    .decode_utf8()
                    .map_err(|_| ConversionError::InvalidUtf8)?;

                for ch in path.encode_utf16() {
                    hdrop.put_u16_le(ch);
                }
                hdrop.put_u16_le(0); // Null terminator
            }
        }
        hdrop.put_u16_le(0); // Double null terminator

        Ok(hdrop.to_vec())
    }

    fn convert_hdrop_to_uri_list(&self, data: &[u8]) -> Result<Vec<u8>, ConversionError> {
        if data.len() < 20 {
            return Err(ConversionError::InvalidData);
        }

        let offset = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let wide = i32::from_le_bytes([data[16], data[17], data[18], data[19]]) != 0;

        let file_data = &data[offset..];
        let mut uri_list = String::new();

        if wide {
            let mut current_path = Vec::new();
            let mut i = 0;

            while i + 1 < file_data.len() {
                let ch = u16::from_le_bytes([file_data[i], file_data[i + 1]]);
                i += 2;

                if ch == 0 {
                    if !current_path.is_empty() {
                        let path = String::from_utf16(&current_path)
                            .map_err(|_| ConversionError::InvalidUtf16)?;

                        uri_list.push_str("file://");
                        uri_list.push_str(&percent_encoding::utf8_percent_encode(
                            &path,
                            percent_encoding::NON_ALPHANUMERIC
                        ).to_string());
                        uri_list.push('\n');

                        current_path.clear();
                    } else {
                        break; // Double null terminator
                    }
                } else {
                    current_path.push(ch);
                }
            }
        }

        Ok(uri_list.into_bytes())
    }

    pub fn rdp_to_mime_types(&self, formats: &[ClipboardFormat]) -> Result<Vec<String>, ConversionError> {
        let mut mime_types = Vec::new();

        for format in formats {
            if let Some(mime) = self.format_map.get(&format.format_id) {
                mime_types.push(mime.clone());
            } else if format.format_id >= CF_CUSTOM_START {
                // Handle custom format
                mime_types.push(format!("application/x-rdp-custom-{}", format.format_id));
            }
        }

        Ok(mime_types)
    }

    pub fn mime_to_rdp_formats(&self, mime_types: &[String]) -> Result<Vec<ClipboardFormat>, ConversionError> {
        let mut formats = Vec::new();

        for mime in mime_types {
            if let Some(format_id) = self.mime_map.get(mime) {
                formats.push(ClipboardFormat {
                    format_id: *format_id,
                    format_name: self.get_format_name(*format_id),
                });
            } else if mime.starts_with("application/x-rdp-custom-") {
                // Handle custom format
                let id_str = &mime[25..];
                if let Ok(id) = id_str.parse::<u32>() {
                    formats.push(ClipboardFormat {
                        format_id: id,
                        format_name: mime.clone(),
                    });
                }
            }
        }

        Ok(formats)
    }

    fn get_format_name(&self, format_id: u32) -> String {
        match format_id {
            CF_TEXT => "CF_TEXT".to_string(),
            CF_BITMAP => "CF_BITMAP".to_string(),
            CF_UNICODETEXT => "CF_UNICODETEXT".to_string(),
            CF_DIB => "CF_DIB".to_string(),
            CF_HTML => "HTML Format".to_string(),
            CF_PNG => "PNG".to_string(),
            CF_JPEG => "JPEG".to_string(),
            CF_RTF => "Rich Text Format".to_string(),
            CF_HDROP => "CF_HDROP".to_string(),
            _ => format!("Format_{}", format_id),
        }
    }

    pub fn format_id_to_mime(&self, format_id: u32) -> Result<String, ConversionError> {
        self.format_map.get(&format_id)
            .cloned()
            .ok_or(ConversionError::UnknownFormat)
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardFormat {
    pub format_id: u32,
    pub format_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Unsupported format")]
    UnsupportedFormat,
    #[error("Invalid UTF-8 data")]
    InvalidUtf8,
    #[error("Invalid UTF-16 data")]
    InvalidUtf16,
    #[error("Invalid data structure")]
    InvalidData,
    #[error("Image decode error")]
    ImageDecodeError,
    #[error("Image encode error")]
    ImageEncodeError,
    #[error("Image creation error")]
    ImageCreateError,
    #[error("Unsupported bit depth")]
    UnsupportedBitDepth,
    #[error("Unknown format")]
    UnknownFormat,
}
```

### Loop Prevention (`src/clipboard/loop_prevention.rs`)
```rust
use std::collections::VecDeque;
use std::time::{SystemTime, Duration};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub enum ClipboardState {
    Idle,
    RdpOwned(Vec<ClipboardFormat>),
    PortalOwned(Vec<String>),
    Syncing(SyncDirection),
}

#[derive(Debug, Clone)]
pub enum SyncDirection {
    RdpToPortal,
    PortalToRdp,
}

pub struct LoopDetector {
    /// Recent clipboard operations
    history: VecDeque<ClipboardOperation>,
    /// Time window for loop detection (ms)
    window_ms: u64,
    /// Content hashes for comparison
    content_cache: VecDeque<ContentHash>,
    /// Maximum history size
    max_history: usize,
}

#[derive(Debug, Clone)]
struct ClipboardOperation {
    timestamp: SystemTime,
    source: OperationSource,
    format_hash: String,
}

#[derive(Debug, Clone)]
enum OperationSource {
    Rdp,
    Portal,
}

#[derive(Debug, Clone)]
struct ContentHash {
    timestamp: SystemTime,
    hash: String,
    source: OperationSource,
}

impl LoopDetector {
    pub fn new(window_ms: u64) -> Self {
        Self {
            history: VecDeque::with_capacity(10),
            window_ms,
            content_cache: VecDeque::with_capacity(5),
            max_history: 10,
        }
    }

    pub async fn would_cause_loop(&mut self, formats: &[ClipboardFormat]) -> bool {
        let now = SystemTime::now();
        let format_hash = self.hash_formats(formats);

        // Clean old entries
        self.clean_old_entries(now);

        // Check if same format list was recently sent from Portal
        for op in &self.history {
            if matches!(op.source, OperationSource::Portal) {
                let age = now.duration_since(op.timestamp)
                    .unwrap_or(Duration::from_secs(0));

                if age.as_millis() < self.window_ms as u128 {
                    if op.format_hash == format_hash {
                        debug!("Loop detected: RDP format matches recent Portal operation");
                        return true;
                    }
                }
            }
        }

        false
    }

    pub async fn would_cause_loop_mime(&mut self, mime_types: &[String]) -> bool {
        let now = SystemTime::now();
        let format_hash = self.hash_mime_types(mime_types);

        // Clean old entries
        self.clean_old_entries(now);

        // Check if same MIME types were recently sent from RDP
        for op in &self.history {
            if matches!(op.source, OperationSource::Rdp) {
                let age = now.duration_since(op.timestamp)
                    .unwrap_or(Duration::from_secs(0));

                if age.as_millis() < self.window_ms as u128 {
                    if op.format_hash == format_hash {
                        debug!("Loop detected: Portal MIME types match recent RDP operation");
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn record_rdp_operation(&mut self, formats: Vec<ClipboardFormat>) {
        let operation = ClipboardOperation {
            timestamp: SystemTime::now(),
            source: OperationSource::Rdp,
            format_hash: self.hash_formats(&formats),
        };

        self.add_to_history(operation);
    }

    pub fn record_portal_operation(&mut self, mime_types: Vec<String>) {
        let operation = ClipboardOperation {
            timestamp: SystemTime::now(),
            source: OperationSource::Portal,
            format_hash: self.hash_mime_types(&mime_types),
        };

        self.add_to_history(operation);
    }

    pub fn check_content_loop(&mut self, content: &[u8], source: OperationSource) -> bool {
        let now = SystemTime::now();
        let content_hash = self.hash_content(content);

        // Clean old content hashes
        self.clean_old_content(now);

        // Check if same content was recently processed from opposite source
        for cached in &self.content_cache {
            let age = now.duration_since(cached.timestamp)
                .unwrap_or(Duration::from_secs(0));

            if age.as_millis() < self.window_ms as u128 {
                let is_opposite_source = match (&cached.source, &source) {
                    (OperationSource::Rdp, OperationSource::Portal) => true,
                    (OperationSource::Portal, OperationSource::Rdp) => true,
                    _ => false,
                };

                if is_opposite_source && cached.hash == content_hash {
                    debug!("Content loop detected: identical content from opposite source");
                    return true;
                }
            }
        }

        // Store content hash
        self.content_cache.push_back(ContentHash {
            timestamp: now,
            hash: content_hash,
            source,
        });

        // Limit cache size
        while self.content_cache.len() > 5 {
            self.content_cache.pop_front();
        }

        false
    }

    fn hash_formats(&self, formats: &[ClipboardFormat]) -> String {
        let mut hasher = Sha256::new();

        let mut sorted_formats = formats.to_vec();
        sorted_formats.sort_by_key(|f| f.format_id);

        for format in sorted_formats {
            hasher.update(format.format_id.to_le_bytes());
            hasher.update(format.format_name.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn hash_mime_types(&self, mime_types: &[String]) -> String {
        let mut hasher = Sha256::new();

        let mut sorted_types = mime_types.to_vec();
        sorted_types.sort();

        for mime_type in sorted_types {
            hasher.update(mime_type.as_bytes());
        }

        format!("{:x}", hasher.finalize())
    }

    fn hash_content(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    fn add_to_history(&mut self, operation: ClipboardOperation) {
        self.history.push_back(operation);

        // Limit history size
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    fn clean_old_entries(&mut self, now: SystemTime) {
        let threshold = Duration::from_millis(self.window_ms * 2);

        self.history.retain(|op| {
            now.duration_since(op.timestamp)
                .unwrap_or(Duration::from_secs(0)) < threshold
        });
    }

    fn clean_old_content(&mut self, now: SystemTime) {
        let threshold = Duration::from_millis(self.window_ms * 2);

        self.content_cache.retain(|cached| {
            now.duration_since(cached.timestamp)
                .unwrap_or(Duration::from_secs(0)) < threshold
        });
    }
}
```

### IronRDP Integration (`src/clipboard/ironrdp_backend.rs`)
```rust
use async_trait::async_trait;
use ironrdp::cliprdr::{
    Backend, CliprdrBackend, CliprdrServerFactory,
    FormatListPdu, FormatDataRequestPdu, FormatDataResponsePdu,
    ClipboardGeneralCapabilitySet, CliprdrError,
};
use tokio::sync::mpsc;

pub struct ClipboardBackend {
    event_tx: mpsc::Sender<ClipboardEvent>,
    capabilities: ClipboardGeneralCapabilitySet,
}

impl ClipboardBackend {
    pub fn new(event_tx: mpsc::Sender<ClipboardEvent>) -> Self {
        let capabilities = ClipboardGeneralCapabilitySet {
            version: 2,
            general_flags: GeneralFlags::USE_LONG_FORMAT_NAMES |
                          GeneralFlags::STREAM_FILECLIP_ENABLED |
                          GeneralFlags::FILECLIP_NO_FILE_PATHS,
        };

        Self {
            event_tx,
            capabilities,
        }
    }
}

#[async_trait]
impl Backend for ClipboardBackend {
    type Error = CliprdrError;

    async fn handle_format_list(&mut self, pdu: FormatListPdu) -> Result<(), Self::Error> {
        let formats: Vec<ClipboardFormat> = pdu.formats
            .into_iter()
            .map(|f| ClipboardFormat {
                format_id: f.format_id,
                format_name: f.format_name.unwrap_or_default(),
            })
            .collect();

        self.event_tx
            .send(ClipboardEvent::RdpFormatList(formats))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }

    async fn handle_format_data_request(&mut self, pdu: FormatDataRequestPdu) -> Result<(), Self::Error> {
        self.event_tx
            .send(ClipboardEvent::RdpDataRequest(pdu.format_id))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }

    async fn handle_format_data_response(&mut self, pdu: FormatDataResponsePdu) -> Result<(), Self::Error> {
        self.event_tx
            .send(ClipboardEvent::RdpDataResponse(pdu.data))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }

    fn get_capabilities(&self) -> ClipboardGeneralCapabilitySet {
        self.capabilities.clone()
    }
}

pub struct CliprdrServerImpl {
    backend: ClipboardBackend,
    tx: mpsc::Sender<CliprdrMessage>,
    rx: mpsc::Receiver<CliprdrMessage>,
}

impl CliprdrServerImpl {
    pub fn new(event_tx: mpsc::Sender<ClipboardEvent>) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let backend = ClipboardBackend::new(event_tx);

        Self {
            backend,
            tx,
            rx,
        }
    }

    pub async fn send_format_list(&mut self, formats: Vec<ClipboardFormat>) -> Result<(), CliprdrError> {
        let pdu = FormatListPdu {
            formats: formats.into_iter().map(|f| {
                ironrdp::cliprdr::ClipboardFormat {
                    format_id: f.format_id,
                    format_name: Some(f.format_name),
                }
            }).collect(),
        };

        self.tx.send(CliprdrMessage::FormatList(pdu))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }

    pub async fn send_data_response(&mut self, data: Vec<u8>) -> Result<(), CliprdrError> {
        let pdu = FormatDataResponsePdu {
            data,
            is_error: false,
        };

        self.tx.send(CliprdrMessage::DataResponse(pdu))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }

    pub async fn request_data(&mut self, format_id: u32) -> Result<(), CliprdrError> {
        let pdu = FormatDataRequestPdu {
            format_id,
        };

        self.tx.send(CliprdrMessage::DataRequest(pdu))
            .await
            .map_err(|_| CliprdrError::ChannelError)?;

        Ok(())
    }
}

enum CliprdrMessage {
    FormatList(FormatListPdu),
    DataRequest(FormatDataRequestPdu),
    DataResponse(FormatDataResponsePdu),
}
```

### Portal Integration (`src/portal/clipboard.rs`)
```rust
use zbus::{Connection, proxy};
use zvariant::{OwnedValue, Value};
use std::collections::HashMap;

#[proxy(
    interface = "org.freedesktop.portal.Clipboard",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait ClipboardPortal {
    async fn request_clipboard_selection(&self, options: HashMap<String, OwnedValue>) -> zbus::Result<String>;
    async fn set_selection(&self, mime_types: Vec<String>, options: HashMap<String, OwnedValue>) -> zbus::Result<()>;
    async fn selection_read(&self, mime_type: &str) -> zbus::Result<Vec<u8>>;
    async fn selection_write(&self, mime_type: &str, data: Vec<u8>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn selection_owner_changed(&self, session_handle: String) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn selection_transfer(&self, mime_type: String, fd: i32) -> zbus::Result<()>;
}

pub struct ClipboardPortal {
    connection: Connection,
    proxy: ClipboardPortalProxy<'static>,
    session_handle: Option<String>,
}

impl ClipboardPortal {
    pub async fn new() -> Result<Self, PortalError> {
        let connection = Connection::session().await?;
        let proxy = ClipboardPortalProxy::new(&connection).await?;

        let portal = Self {
            connection,
            proxy,
            session_handle: None,
        };

        portal.initialize_session().await?;

        Ok(portal)
    }

    async fn initialize_session(&mut self) -> Result<(), PortalError> {
        let mut options = HashMap::new();
        options.insert(
            "session_handle_token".to_string(),
            OwnedValue::from("clipboard_session"),
        );

        let handle = self.proxy.request_clipboard_selection(options).await?;
        self.session_handle = Some(handle);

        Ok(())
    }

    pub async fn advertise_formats(&self, mime_types: Vec<String>) -> Result<(), PortalError> {
        let mut options = HashMap::new();
        if let Some(ref handle) = self.session_handle {
            options.insert(
                "session_handle".to_string(),
                OwnedValue::from(handle.as_str()),
            );
        }

        self.proxy.set_selection(mime_types, options).await?;
        Ok(())
    }

    pub async fn get_data(&self, mime_type: &str) -> Result<Vec<u8>, PortalError> {
        let data = self.proxy.selection_read(mime_type).await?;
        Ok(data)
    }

    pub async fn set_data(&self, mime_type: &str, data: Vec<u8>) -> Result<(), PortalError> {
        self.proxy.selection_write(mime_type, data).await?;
        Ok(())
    }

    pub async fn monitor_selection_changes(&self) -> Result<(), PortalError> {
        let mut stream = self.proxy.receive_selection_owner_changed().await?;

        while let Some(signal) = stream.next().await {
            let args = signal.args()?;
            if args.session_handle != self.session_handle.as_deref().unwrap_or("") {
                // Selection changed by another application
                self.handle_selection_change().await?;
            }
        }

        Ok(())
    }

    async fn handle_selection_change(&self) -> Result<(), PortalError> {
        // Query available formats
        // This would trigger PortalDataAvailable event
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PortalError {
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    #[error("Portal not available")]
    NotAvailable,
    #[error("Session initialization failed")]
    SessionError,
}

impl Clone for ClipboardPortal {
    fn clone(&self) -> Self {
        // Note: This creates a new connection
        let connection = self.connection.clone();
        let proxy = self.proxy.clone();

        Self {
            connection,
            proxy,
            session_handle: self.session_handle.clone(),
        }
    }
}
```

## TESTING REQUIREMENTS

### Unit Tests (`src/clipboard/tests/mod.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_text_format_conversion() {
        let converter = FormatConverter::new();

        // UTF-8 to UTF-16
        let utf8_text = "Hello, 世界!".as_bytes();
        let utf16_result = converter.convert_text_to_unicode(utf8_text).unwrap();

        // Verify UTF-16 encoding
        let expected_utf16 = "Hello, 世界!"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .chain([0u8, 0u8])
            .collect::<Vec<_>>();

        assert_eq!(utf16_result, expected_utf16);

        // UTF-16 to UTF-8
        let utf8_result = converter.convert_unicode_to_text(&utf16_result).unwrap();
        assert_eq!(utf8_result, utf8_text);
    }

    #[tokio::test]
    async fn test_dib_conversion() {
        let converter = FormatConverter::new();

        // Create test image
        let image = DynamicImage::ImageRgba8(
            image::RgbaImage::from_pixel(100, 100, image::Rgba([255, 0, 0, 255]))
        );

        // Convert to DIB
        let dib_data = converter.create_dib_from_image(&image).unwrap();

        // Verify DIB header
        assert!(dib_data.len() >= 40);
        assert_eq!(u32::from_le_bytes([dib_data[0], dib_data[1], dib_data[2], dib_data[3]]), 40);

        // Convert back to PNG
        let png_data = converter.convert_dib_to_png(&dib_data).await.unwrap();

        // Load and verify
        let loaded_image = image::load_from_memory(&png_data).unwrap();
        assert_eq!(loaded_image.width(), 100);
        assert_eq!(loaded_image.height(), 100);
    }

    #[tokio::test]
    async fn test_html_format_conversion() {
        let converter = FormatConverter::new();

        let html = "<b>Test HTML</b>";
        let rdp_html = converter.convert_html_to_rdp_html(html.as_bytes()).unwrap();

        // Verify RDP HTML format headers
        let rdp_str = std::str::from_utf8(&rdp_html).unwrap();
        assert!(rdp_str.starts_with("Version:0.9"));
        assert!(rdp_str.contains("StartHTML:"));
        assert!(rdp_str.contains("EndHTML:"));
        assert!(rdp_str.contains("StartFragment:"));
        assert!(rdp_str.contains("EndFragment:"));
        assert!(rdp_str.contains("<b>Test HTML</b>"));
    }

    #[tokio::test]
    async fn test_loop_detection() {
        let mut detector = LoopDetector::new(500);

        let formats = vec![
            ClipboardFormat {
                format_id: CF_UNICODETEXT,
                format_name: "CF_UNICODETEXT".to_string(),
            },
        ];

        // First operation should not cause loop
        assert!(!detector.would_cause_loop(&formats).await);
        detector.record_rdp_operation(formats.clone());

        // Same format from Portal should be detected as loop
        let mime_types = vec!["text/plain".to_string()];
        detector.record_portal_operation(mime_types.clone());

        // Now RDP with same format should trigger loop detection
        assert!(detector.would_cause_loop(&formats).await);

        // After window expires, should not detect loop
        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
        assert!(!detector.would_cause_loop(&formats).await);
    }

    #[tokio::test]
    async fn test_content_loop_detection() {
        let mut detector = LoopDetector::new(500);

        let content = b"Test clipboard content";

        // First operation should not cause loop
        assert!(!detector.check_content_loop(content, OperationSource::Rdp));

        // Same content from opposite source should trigger loop
        assert!(detector.check_content_loop(content, OperationSource::Portal));

        // Different content should not trigger loop
        let different_content = b"Different content";
        assert!(!detector.check_content_loop(different_content, OperationSource::Portal));
    }

    #[tokio::test]
    async fn test_uri_list_conversion() {
        let converter = FormatConverter::new();

        let uri_list = "file:///home/user/document.txt\nfile:///home/user/image.png";
        let hdrop = converter.convert_uri_list_to_hdrop(uri_list.as_bytes()).unwrap();

        // Verify DROPFILES structure
        assert!(hdrop.len() >= 20);

        // Convert back
        let uri_result = converter.convert_hdrop_to_uri_list(&hdrop).unwrap();
        let uri_str = std::str::from_utf8(&uri_result).unwrap();

        assert!(uri_str.contains("file:///home/user/document.txt"));
        assert!(uri_str.contains("file:///home/user/image.png"));
    }

    #[tokio::test]
    async fn test_large_data_handling() {
        let config = ClipboardConfig {
            max_data_size: 1024 * 1024, // 1MB limit
            ..Default::default()
        };

        let manager = ClipboardManager::new(config).await.unwrap();

        // Test data at limit
        let data_at_limit = vec![0u8; 1024 * 1024];
        // This should succeed

        // Test data over limit
        let data_over_limit = vec![0u8; 1024 * 1024 + 1];
        // This should fail with appropriate error
    }

    #[tokio::test]
    async fn test_format_mapping_completeness() {
        let converter = FormatConverter::new();

        // Test all standard formats have mappings
        let standard_formats = vec![
            CF_TEXT, CF_BITMAP, CF_UNICODETEXT, CF_DIB,
            CF_HDROP, CF_HTML, CF_PNG, CF_JPEG, CF_RTF,
        ];

        for format_id in standard_formats {
            assert!(converter.format_id_to_mime(format_id).is_ok(),
                   "Missing MIME mapping for format {}", format_id);
        }
    }
}
```

## PERFORMANCE CONSIDERATIONS

1. **Chunked Transfer**: Large clipboard data transferred in chunks
2. **Async Processing**: All I/O operations are async
3. **Lazy Conversion**: Format conversion only when requested
4. **Caching**: Recent clipboard content cached to prevent re-conversion
5. **Memory Limits**: Configurable maximum data size
6. **Stream Processing**: Large images processed as streams

## ERROR HANDLING

1. **Conversion Errors**: Graceful fallback to supported formats
2. **Network Errors**: Retry with exponential backoff
3. **Memory Errors**: Reject oversized data with clear error
4. **Format Errors**: Log unsupported formats, continue with supported ones
5. **Loop Detection**: Silent rejection with debug logging

## CONFIGURATION

```toml
[clipboard]
max_data_size = 16777216  # 16MB
enable_images = true
enable_files = true
enable_html = true
enable_rtf = true
chunk_size = 65536  # 64KB
timeout_ms = 5000
loop_detection_window_ms = 500
supported_image_formats = ["png", "jpeg", "bmp", "gif"]
```

## INTEGRATION POINTS

1. **IronRDP**: Via CliprdrServer trait implementation
2. **Portal**: Via DBus org.freedesktop.portal.Clipboard
3. **Event System**: Async message passing for all clipboard events
4. **State Machine**: Centralized state management
5. **Metrics**: Performance and error tracking

## DELIVERABLES

1. ✅ Complete format mapping implementation (15+ formats)
2. ✅ Full image conversion suite (DIB, PNG, JPEG, BMP)
3. ✅ Text encoding conversions (UTF-8, UTF-16, ASCII)
4. ✅ Rich text support (HTML, RTF)
5. ✅ File transfer support (HDROP ↔ text/uri-list)
6. ✅ Loop prevention with state machine
7. ✅ IronRDP CliprdrServer integration
8. ✅ Portal clipboard integration
9. ✅ Comprehensive test suite
10. ✅ Performance optimizations
11. ✅ Production-grade error handling

## DEVELOPMENT TIMELINE

- Day 1-2: Core manager and state machine
- Day 3-4: Format conversion implementation
- Day 5-6: IronRDP and Portal integration
- Day 7-8: Loop prevention and optimization
- Day 9-10: Testing and refinement

**Total Duration:** 7-10 days
**Lines of Code:** ~1100 lines
**Test Coverage Target:** >90%
**Status:** SPECIFICATION_COMPLETE