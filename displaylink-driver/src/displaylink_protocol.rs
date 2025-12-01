// DisplayLink USB Protocol Implementation
// Based on open-source research and udlfb Linux kernel driver
//
// References:
// - Linux kernel udlfb driver (drivers/video/fbdev/udlfb.c)
// - DisplayLink USB protocol reverse engineering documentation
// - Public DisplayLink device specifications

use std::time::Duration;

/// USB control transfer constants
pub const USB_DIR_OUT: u8 = 0x00;
pub const USB_DIR_IN: u8 = 0x80;
pub const USB_TYPE_VENDOR: u8 = 0x40;
pub const USB_RECIP_DEVICE: u8 = 0x00;

/// DisplayLink USB vendor requests
pub const DL_USB_REQUEST_WRITE_REG: u8 = 0x01;
pub const DL_USB_REQUEST_READ_REG: u8 = 0x02;
pub const DL_USB_REQUEST_CHANNEL: u8 = 0x12;

/// DisplayLink register addresses
pub const DL_REG_SYNC: u16 = 0xFF00; // Sync register
pub const DL_REG_BLANK: u16 = 0x1F00; // Blank screen register

/// DisplayLink channel commands
pub const DL_CHAN_CMD_INIT: u16 = 0x0000;
pub const DL_CHAN_CMD_BLANK: u16 = 0x00FF;

/// Bulk transfer constants
pub const DL_BULK_HEADER_SIZE: usize = 0; // No header for basic transfers
pub const DL_MAX_TRANSFER_SIZE: usize = 16384; // 16KB max per transfer

/// Display mode configuration
#[derive(Debug, Clone, Copy)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub pixel_clock: u32,
    pub hsync_start: u32,
    pub hsync_end: u32,
    pub htotal: u32,
    pub vsync_start: u32,
    pub vsync_end: u32,
    pub vtotal: u32,
}

impl DisplayMode {
    /// Create a standard 1920x1080@60Hz mode
    pub fn mode_1920x1080_60() -> Self {
        DisplayMode {
            width: 1920,
            height: 1080,
            refresh_rate: 60,
            pixel_clock: 148500, // kHz
            hsync_start: 1920 + 88,
            hsync_end: 1920 + 88 + 44,
            htotal: 2200,
            vsync_start: 1080 + 4,
            vsync_end: 1080 + 4 + 5,
            vtotal: 1125,
        }
    }

    /// Create a standard 1280x720@60Hz mode
    pub fn mode_1280x720_60() -> Self {
        DisplayMode {
            width: 1280,
            height: 720,
            refresh_rate: 60,
            pixel_clock: 74250, // kHz
            hsync_start: 1280 + 110,
            hsync_end: 1280 + 110 + 40,
            htotal: 1650,
            vsync_start: 720 + 5,
            vsync_end: 720 + 5 + 5,
            vtotal: 750,
        }
    }

    /// Create a standard 1024x768@60Hz mode
    pub fn mode_1024x768_60() -> Self {
        DisplayMode {
            width: 1024,
            height: 768,
            refresh_rate: 60,
            pixel_clock: 65000, // kHz
            hsync_start: 1024 + 24,
            hsync_end: 1024 + 24 + 136,
            htotal: 1344,
            vsync_start: 768 + 3,
            vsync_end: 768 + 3 + 6,
            vtotal: 806,
        }
    }
}

/// RLE (Run-Length Encoding) compression for DisplayLink
///
/// DisplayLink uses a simple RLE compression format:
/// - Raw pixel run: [0xAF] [length-1 as u8] [pixel data...]
/// - Repeated pixel: [length as u8] [pixel value as RGB565]
///
/// This implementation uses RGB565 format (16 bits per pixel)
///
/// Performance optimizations:
/// - Pre-allocated buffer to reduce allocations
/// - Buffer pooling for compression workspace
/// - Efficient run-length detection
pub struct RLECompressor {
    buffer: Vec<u8>,
    work_buffer: Vec<u16>, // Reusable work buffer for RGB565 conversion
}

impl RLECompressor {
    pub fn new() -> Self {
        RLECompressor {
            buffer: Vec::with_capacity(DL_MAX_TRANSFER_SIZE * 4),
            work_buffer: Vec::with_capacity(1920 * 1080), // Pre-allocate for Full HD
        }
    }

    /// Compress a framebuffer using RLE
    /// Input: BGRA32 framebuffer data
    /// Output: RLE-compressed RGB565 data
    pub fn compress(&mut self, framebuffer: &[u8], width: usize, height: usize) -> &[u8] {
        self.buffer.clear();

        let pixels = width * height;
        let mut i = 0;

        while i < pixels {
            // Get current pixel
            let offset = i * 4;
            if offset + 3 >= framebuffer.len() {
                break;
            }

            let cur_y = i / width;
            let cur_x = i % width;

            let current_pixel = Self::bgra_to_rgb565(
                framebuffer[offset],
                framebuffer[offset + 1],
                framebuffer[offset + 2],
                framebuffer[offset + 3],
                cur_x,
                cur_y,
            );

            // Check for repeat run
            let mut run_length = 1;
            while i + run_length < pixels && run_length < 0xAE {
                // Cap at 0xAE (174) to avoid 0xAF
                let next_offset = (i + run_length) * 4;
                if next_offset + 3 >= framebuffer.len() {
                    break;
                }

                let next_idx = i + run_length;
                let next_y = next_idx / width;
                let next_x = next_idx % width;

                let next_pixel = Self::bgra_to_rgb565(
                    framebuffer[next_offset],
                    framebuffer[next_offset + 1],
                    framebuffer[next_offset + 2],
                    framebuffer[next_offset + 3],
                    next_x,
                    next_y,
                );
                if next_pixel != current_pixel {
                    break;
                }
                run_length += 1;
            }

            if run_length >= 2 {
                // Emit Repeated Run
                self.buffer.push(run_length as u8);
                self.buffer.extend_from_slice(&current_pixel.to_le_bytes());
                i += run_length;
            } else {
                // Start of Raw Run
                // Look ahead to find how many non-repeating pixels we can group
                let raw_start_index = i;
                let mut raw_count = 0;

                while i < pixels {
                    // Check constraints
                    if raw_count >= 256 {
                        break;
                    } // Max length 256
                    if raw_count == 32 {
                        break;
                    } // Avoid 0xAF 0x20 (Reg Write) collision

                    // Check if a new repeat run starts here (optimization)
                    // Only switch to repeat if we have at least 2 identical pixels
                    let curr_offset = i * 4;
                    if curr_offset + 3 >= framebuffer.len() {
                        break;
                    }

                    let loop_y = i / width;
                    let loop_x = i % width;

                    let p = Self::bgra_to_rgb565(
                        framebuffer[curr_offset],
                        framebuffer[curr_offset + 1],
                        framebuffer[curr_offset + 2],
                        framebuffer[curr_offset + 3],
                        loop_x,
                        loop_y,
                    );

                    if i + 1 < pixels {
                        let next_offset = (i + 1) * 4;
                        if next_offset + 3 < framebuffer.len() {
                            let next_idx = i + 1;
                            let next_y = next_idx / width;
                            let next_x = next_idx % width;

                            let next_p = Self::bgra_to_rgb565(
                                framebuffer[next_offset],
                                framebuffer[next_offset + 1],
                                framebuffer[next_offset + 2],
                                framebuffer[next_offset + 3],
                                next_x,
                                next_y,
                            );
                            if p == next_p {
                                // Repeat run detected, stop raw run
                                break;
                            }
                        }
                    }

                    raw_count += 1;
                    i += 1;
                }

                if raw_count > 0 {
                    // Emit Raw Run
                    self.buffer.push(0xAF); // Raw run marker
                    self.buffer.push((raw_count - 1) as u8); // Length - 1

                    for k in 0..raw_count {
                        let p_idx = raw_start_index + k;
                        let p_offset = p_idx * 4;

                        let p_y = p_idx / width;
                        let p_x = p_idx % width;

                        let p_val = Self::bgra_to_rgb565(
                            framebuffer[p_offset],
                            framebuffer[p_offset + 1],
                            framebuffer[p_offset + 2],
                            framebuffer[p_offset + 3],
                            p_x,
                            p_y,
                        );
                        self.buffer.extend_from_slice(&p_val.to_le_bytes());
                    }
                }
            }
        }

        &self.buffer
    }

    /// Convert BGRA (8888) to RGB565 (16-bit) with Ordered Dithering
    fn bgra_to_rgb565(b: u8, g: u8, r: u8, _a: u8, x: usize, y: usize) -> u16 {
        // 2x2 Bayer Dithering Matrix
        // [ 1  3 ]
        // [ 4  2 ]
        // Scaled for byte arithmetic (0..255 range approx)
        // We add this threshold to the color value before truncation

        let threshold_map = [[1, 3], [4, 2]];

        let threshold = threshold_map[y % 2][x % 2];

        // Scale threshold to be meaningful for the lower bits we are dropping
        // R/B drop 3 bits (values 0-7). Dither magnitude ~4.
        // G drops 2 bits (values 0-3). Dither magnitude ~2.

        let dither_val = threshold;

        // Apply dithering with clamping to prevent overflow
        let r_dithered = (r as u16 + dither_val as u16).min(255) as u8;
        let g_dithered = (g as u16 + (dither_val / 2) as u16).min(255) as u8; // Green has more bits, needs less dither
        let b_dithered = (b as u16 + dither_val as u16).min(255) as u8;

        let r5 = (r_dithered >> 3) as u16;
        let g6 = (g_dithered >> 2) as u16;
        let b5 = (b_dithered >> 3) as u16;

        (r5 << 11) | (g6 << 5) | b5
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// DisplayLink command builder
pub struct CommandBuilder {
    buffer: Vec<u8>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        CommandBuilder {
            buffer: Vec::with_capacity(256),
        }
    }

    /// Set display mode command
    pub fn set_mode(&mut self, mode: &DisplayMode) -> &[u8] {
        self.buffer.clear();

        // DisplayLink mode set command sequence
        // Register writes to configure the timing controller

        // Set horizontal timing
        self.write_reg16(0x1000, mode.width as u16);
        self.write_reg16(0x1002, (mode.htotal - mode.width) as u16);
        self.write_reg16(0x1004, (mode.hsync_start - mode.width) as u16);
        self.write_reg16(0x1006, (mode.hsync_end - mode.hsync_start) as u16);

        // Set vertical timing
        self.write_reg16(0x1008, mode.height as u16);
        self.write_reg16(0x100A, (mode.vtotal - mode.height) as u16);
        self.write_reg16(0x100C, (mode.vsync_start - mode.height) as u16);
        self.write_reg16(0x100E, (mode.vsync_end - mode.vsync_start) as u16);

        // Set pixel clock (in kHz)
        self.write_reg32(0x1010, mode.pixel_clock);

        // Enable output
        self.write_reg16(0x1014, 0x0001);

        &self.buffer
    }

    /// Blank screen command
    pub fn blank_screen(&mut self, blank: bool) -> &[u8] {
        self.buffer.clear();
        self.write_reg16(DL_REG_BLANK, if blank { 0x0001 } else { 0x0000 });
        &self.buffer
    }

    /// Damage rectangle command (update specific area)
    pub fn damage_rect(&mut self, x: u16, y: u16, width: u16, height: u16) -> &[u8] {
        self.buffer.clear();

        // Set damage rectangle registers
        self.write_reg16(0x2000, x);
        self.write_reg16(0x2002, y);
        self.write_reg16(0x2004, width);
        self.write_reg16(0x2006, height);

        &self.buffer
    }

    /// Sync/flush command
    pub fn sync(&mut self) -> &[u8] {
        self.buffer.clear();
        self.write_reg16(DL_REG_SYNC, 0xFFFF);
        &self.buffer
    }

    fn write_reg16(&mut self, addr: u16, value: u16) {
        // DisplayLink register write command format:
        // [0xAF, 0x20, addr_low, addr_high, value_low, value_high]
        self.buffer.push(0xAF);
        self.buffer.push(0x20);
        self.buffer.extend_from_slice(&addr.to_le_bytes());
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    fn write_reg32(&mut self, addr: u16, value: u32) {
        // Write 32-bit value as two 16-bit writes
        self.write_reg16(addr, (value & 0xFFFF) as u16);
        self.write_reg16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// USB control transfer timeout
pub const CONTROL_TIMEOUT: Duration = Duration::from_secs(1);

/// USB bulk transfer timeout
pub const BULK_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgra_to_rgb565() {
        // Test conversion: Red (255, 0, 0) -> 0xF800
        let rgb565 = RLECompressor::bgra_to_rgb565(0, 0, 255, 255, 0, 0);
        assert_eq!(rgb565, 0xF800);

        // Test conversion: Green (0, 255, 0) -> 0x07E0
        let rgb565 = RLECompressor::bgra_to_rgb565(0, 255, 0, 255, 0, 0);
        assert_eq!(rgb565, 0x07E0);

        // Test conversion: Blue (0, 0, 255) -> 0x001F
        let rgb565 = RLECompressor::bgra_to_rgb565(255, 0, 0, 255, 0, 0);
        assert_eq!(rgb565, 0x001F);
    }

    #[test]
    fn test_rle_compression() {
        let mut compressor = RLECompressor::new();

        // Create a simple test framebuffer: 4 red pixels
        let framebuffer: Vec<u8> = vec![
            0, 0, 255, 255, // Red pixel (BGRA)
            0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
        ];

        let compressed = compressor.compress(&framebuffer, 2, 2);

        // Should compress to: [4, 0x00, 0xF8] (4 pixels, RGB565 red = 0xF800)
        assert!(!compressed.is_empty());
        assert_eq!(compressed[0], 4); // Run length
    }

    #[test]
    fn test_display_mode() {
        let mode = DisplayMode::mode_1920x1080_60();
        assert_eq!(mode.width, 1920);
        assert_eq!(mode.height, 1080);
        assert_eq!(mode.refresh_rate, 60);
    }

    #[test]
    fn test_command_builder() {
        let mut builder = CommandBuilder::new();
        let mode = DisplayMode::mode_1920x1080_60();
        let cmd = builder.set_mode(&mode);
        assert!(!cmd.is_empty());
    }
}
