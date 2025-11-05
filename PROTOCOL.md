# DisplayLink USB Protocol Documentation

This document describes the reverse-engineered DisplayLink USB protocol implementation used in the STARDRIVE driver.

## Overview

DisplayLink devices use a proprietary USB protocol for transmitting display data. This implementation is based on:
- Analysis of the udlfb Linux kernel driver
- Public DisplayLink device specifications
- USB packet capture analysis
- Open-source reverse engineering efforts

## USB Device Configuration

### Device Information
- **Vendor ID**: 0x17E9 (DisplayLink)
- **Product ID**: 0x4307 (StarTech USB35DOCK)
- **Class**: Vendor-specific
- **Interfaces**:
  - Interface 0 (MI_00): Display adapter
  - Interface 5 (MI_05): Network adapter (CDC NCM)

### Endpoints
- **Bulk OUT**: 0x01 (Display data transmission)
- **Bulk IN**: 0x81 (Device responses/acknowledgments)

## Protocol Layers

### 1. Control Transfers

DisplayLink uses vendor-specific control transfers for device management:

#### Channel Initialization
```
Request Type: 0x40 (USB_DIR_OUT | USB_TYPE_VENDOR | USB_RECIP_DEVICE)
Request: 0x12 (DL_USB_REQUEST_CHANNEL)
Value: 0x0000 (DL_CHAN_CMD_INIT)
Index: 0x0000
Data: []
```

Purpose: Initialize the DisplayLink communication channel.

#### Register Read
```
Request Type: 0xC0 (USB_DIR_IN | USB_TYPE_VENDOR | USB_RECIP_DEVICE)
Request: 0x02 (DL_USB_REQUEST_READ_REG)
Value: [register_address]
Index: 0x0000
Data: [read buffer]
```

#### Register Write
```
Request Type: 0x40 (USB_DIR_OUT | USB_TYPE_VENDOR | USB_RECIP_DEVICE)
Request: 0x01 (DL_USB_REQUEST_WRITE_REG)
Value: [register_address]
Index: 0x0000
Data: [write data]
```

### 2. Bulk Transfers

All display data and commands are sent via bulk OUT endpoint 0x01.

#### Command Format

DisplayLink commands are sent as raw binary data in bulk transfers:

**Register Write Command:**
```
Offset | Size | Description
-------|------|------------
0x00   | 1    | Command marker (0xAF)
0x01   | 1    | Command type (0x20 = register write)
0x02   | 2    | Register address (little-endian)
0x04   | 2    | Register value (little-endian)
```

### 3. Display Mode Configuration

Setting a display mode requires writing to multiple timing registers:

#### Timing Registers

| Register | Description |
|----------|-------------|
| 0x1000 | Horizontal active pixels |
| 0x1002 | Horizontal blanking |
| 0x1004 | Horizontal sync start offset |
| 0x1006 | Horizontal sync width |
| 0x1008 | Vertical active lines |
| 0x100A | Vertical blanking |
| 0x100C | Vertical sync start offset |
| 0x100E | Vertical sync width |
| 0x1010 | Pixel clock (32-bit, kHz) |
| 0x1014 | Output enable (0x0001 = enabled) |

#### Example: 1920x1080@60Hz

```rust
write_reg(0x1000, 1920);      // Width
write_reg(0x1002, 280);       // H-blanking (htotal - width)
write_reg(0x1004, 88);        // H-sync start
write_reg(0x1006, 44);        // H-sync width
write_reg(0x1008, 1080);      // Height
write_reg(0x100A, 45);        // V-blanking (vtotal - height)
write_reg(0x100C, 4);         // V-sync start
write_reg(0x100E, 5);         // V-sync width
write_reg(0x1010, 148500);    // Pixel clock (148.5 MHz)
write_reg(0x1014, 0x0001);    // Enable output
```

### 4. Framebuffer Compression

DisplayLink uses Run-Length Encoding (RLE) for framebuffer compression:

#### Format: RGB565

Framebuffers are converted from BGRA32 (32 bits/pixel) to RGB565 (16 bits/pixel):

```
RGB565 = (R[7:3] << 11) | (G[7:2] << 5) | B[7:3]
```

This reduces bandwidth by 50% while maintaining acceptable color quality.

#### RLE Compression Algorithm

Two types of runs:

**1. Repeated Pixel Run:**
```
Offset | Size | Description
-------|------|------------
0x00   | 1    | Run length (2-255)
0x01   | 2    | Pixel value (RGB565, little-endian)
```

Used when 2 or more consecutive pixels have the same color.

**2. Raw Pixel Run:**
```
Offset | Size | Description
-------|------|------------
0x00   | 1    | Raw run marker (0xAF)
0x01   | 1    | Run length - 1 (0 = 1 pixel)
0x02   | N*2  | Raw pixel data (RGB565, little-endian)
```

Used for single pixels or non-repeating sequences.

#### Compression Example

Input (BGRA32):
```
[255,0,0,255] [255,0,0,255] [255,0,0,255] [0,255,0,255]
   Red            Red            Red          Green
```

Output (RLE compressed):
```
[0x03] [0x00, 0xF8]  [0xAF] [0x00] [0xE0, 0x07]
  │      │    │        │      │      │    │
  │      │    │        │      │      └────┴─ Green (0x07E0)
  │      │    │        │      └─ Length-1 (1 pixel)
  │      │    │        └─ Raw run marker
  │      └────┴─ Red pixel (0xF800)
  └─ Run length (3 pixels)
```

### 5. Screen Update Protocol

To update the display, follow this sequence:

1. **Set Damage Rectangle:**
```rust
write_reg(0x2000, x);       // X offset
write_reg(0x2002, y);       // Y offset
write_reg(0x2004, width);   // Update width
write_reg(0x2006, height);  // Update height
```

2. **Send Compressed Framebuffer:**
```
bulk_out(compressed_data);
```

3. **Sync/Flush:**
```rust
write_reg(0xFF00, 0xFFFF);  // Sync command
```

### 6. Display Control

#### Blank Screen
```rust
write_reg(0x1F00, 0x0001);  // Blank
write_reg(0x1F00, 0x0000);  // Unblank
```

#### Power Management (DPMS)

DisplayLink responds to DPMS events from the DRM subsystem but doesn't require explicit USB commands. EVDI handles DPMS through standard DRM mechanisms.

## EVDI Integration

The driver doesn't communicate directly with X11/Wayland. Instead:

1. **EVDI Kernel Module** creates a virtual DRM device (`/dev/dri/cardX`)
2. **Display Server** treats it as a standard DRM display
3. **libevdi** provides framebuffer access to userspace
4. **DisplayLink Driver** (this project) transmits frames via USB

### Data Flow

```
┌──────────────┐
│  X11/Wayland │
└──────┬───────┘
       │ DRM KMS
       ▼
┌──────────────┐
│  EVDI Module │ (Kernel)
└──────┬───────┘
       │ ioctl
       ▼
┌──────────────┐
│   libevdi    │ (Userspace)
└──────┬───────┘
       │ FFI
       ▼
┌──────────────┐
│ Rust Driver  │
└──────┬───────┘
       │ USB Bulk
       ▼
┌──────────────┐
│   Hardware   │
└──────────────┘
```

## Performance Optimizations

### 1. Dirty Rectangle Tracking

EVDI provides dirty rectangle information via `evdi_grab_pixels()`:
```rust
let mut rects: [evdi_rect; 16];
let mut num_rects: i32;
evdi_grab_pixels(handle, rects.as_mut_ptr(), &mut num_rects);
```

Only update changed regions to reduce USB bandwidth.

### 2. Compression Tuning

- **Threshold**: Use raw runs for sequences < 2 identical pixels
- **Chunk Size**: Send data in 16KB chunks (DL_MAX_TRANSFER_SIZE)
- **Async Compression**: Compress in background while previous frame transmits

### 3. USB Transfer Optimization

```rust
// Batch multiple small commands into single bulk transfer
let mut batch = CommandBuilder::new();
batch.damage_rect(x, y, w, h);
batch.append_framebuffer(compressed);
batch.sync();
bulk_out(batch.data());
```

## Known Limitations

### 1. Proprietary Elements

The following aspects are proprietary and not fully documented:

- **Advanced Compression**: DisplayLink may use additional compression beyond RLE
- **Color Space**: Exact color space conversion parameters
- **Firmware Upload**: Some devices require firmware loading (not implemented)
- **Multi-Monitor**: Protocol for multiple displays per device
- **Audio**: Audio-over-USB protocol (if supported)

### 2. Device Variations

Different DisplayLink chipsets may have:
- Different register layouts
- Additional features (rotation, scaling)
- Varying compression support

### 3. Performance Considerations

- **Latency**: Expect 50-100ms frame latency
- **Bandwidth**: USB 3.0 required for high-resolution/high-refresh displays
- **CPU Usage**: RLE compression is CPU-intensive

## Testing Protocol Implementation

### 1. Verify Device Detection

```bash
lsusb | grep 17e9
# Should show: Bus XXX Device YYY: ID 17e9:4307 DisplayLink
```

### 2. Check USB Communication

```bash
sudo usbmon 1 -s 512 | grep 17e9
# Monitor USB packets to/from DisplayLink device
```

### 3. Validate EVDI Connection

```bash
ls /dev/dri/
# Should show new cardX device

xrandr
# Should list virtual display
```

### 4. Test Framebuffer Transfer

Enable debug logging in the driver to see:
```
Compressing framebuffer: 1920x1080
  Compressed 8294400 bytes -> 245760 bytes
  ✓ Framebuffer sent
```

## Further Research

Areas requiring additional reverse engineering:

1. **Huffman/LZ Compression**: DisplayLink DL-3xxx and newer may use advanced compression
2. **Encryption**: Some enterprise docks use encrypted data transfer
3. **Network Adapter**: USB interface 5 protocol (CDC NCM)
4. **Firmware Protocol**: DFU-style firmware update mechanism
5. **HDCP**: Content protection implementation (if any)

## References

- [Linux udlfb driver](https://github.com/torvalds/linux/blob/master/drivers/video/fbdev/udlfb.c)
- [EVDI project](https://github.com/DisplayLink/evdi)
- [USB Video Class spec](https://www.usb.org/document-library/video-class-v15-document-set)
- [DRM KMS documentation](https://www.kernel.org/doc/html/latest/gpu/drm-kms.html)

## License

This documentation is provided for educational and interoperability purposes. DisplayLink and related trademarks are property of Synaptics Inc.
