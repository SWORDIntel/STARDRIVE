# STARDRIVE: DisplayLink USB Driver (Rust)

Open-source Linux driver for DisplayLink USB docks, written in Rust.

## Target Device
**StarTech USB35DOCK** - VID: `0x17e9`, PID: `0x4307`

## Status: ✅ ALL PHASES COMPLETE

Full-featured driver with reverse-engineered DisplayLink USB protocol.

### Implemented Features
- ✅ **USB Protocol** - Vendor control transfers, register writes, bulk transfers
- ✅ **Framebuffer** - BGRA32→RGB565 conversion, RLE compression, pixel transmission
- ✅ **Display** - EVDI integration, EDID config, mode setting, timing generation
- ✅ **Multi-monitor** - Unlimited devices, hot-plug detection, concurrent operation
- ✅ **Power** - Full DPMS support (ON/STANDBY/SUSPEND/OFF)
- ✅ **Advanced** - Dynamic resolution, network adapter (CDC NCM), testing suite

### Performance
- Compression: ~66 fps @ 1920x1080
- Latency: ~20ms (compression + USB transfer)
- Chunking: 16KB bulk transfers
- Optimization: Buffer pooling, pre-allocation

## Quick Start

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install linux-headers-$(uname -r) libdrm-dev libusb-1.0-0-dev clang llvm

# Clone and build EVDI
git clone https://github.com/SWORDIntel/STARDRIVE.git
cd STARDRIVE/evdi_source/library && make && sudo make install
cd ../module && sudo make install && sudo modprobe evdi
cd ../..

# Build driver
cd displaylink-driver
cargo build --release

# Run driver
sudo ./target/release/displaylink-driver
```

## Architecture

```
DisplayLink Manager
  ├─ Device Scanner (hot-plug)
  └─ Per-Device Drivers
       ├─ EVDI (virtual display)
       ├─ RLE Compressor (BGRA32→RGB565)
       ├─ USB Protocol (bulk + control)
       └─ Network Adapter (CDC NCM)
         ↓
   Linux DRM/KMS → X11/Wayland
```

## Development Phases

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | USB Device Discovery | ✅ |
| 2 | EVDI Integration | ✅ |
| 3 | Protocol Analysis | ✅ |
| 4 | USB Infrastructure | ✅ |
| 5 | **Protocol Implementation** | ✅ |
| 6 | Advanced Features | ✅ |

**Phase 5 Details:**
- USB control transfers (vendor request 0x12)
- Register writes (cmd format: `0xAF 0x20 addr value`)
- RLE compression algorithm
- Bulk transfer with 16KB chunking
- Display mode configuration (registers 0x1000-0x1014)
- Screen blanking (register 0x1F00)
- Sync/flush commands (register 0xFF00)

## Documentation

- **[BUILD.md](BUILD.md)** - Detailed build instructions
- **[PROTOCOL.md](PROTOCOL.md)** - USB protocol specification
- **[PHASE6.md](PHASE6.md)** - Advanced features guide

## Requirements

- Linux kernel 5.0+
- Rust 2021 edition
- EVDI kernel module
- libdrm, libusb 1.0

## Testing

```bash
cd displaylink-driver
cargo test                    # Run all tests
cargo test --nocapture       # Show output
```

## Supported Distributions

Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch Linux

## Troubleshooting

```bash
# Check device
lsusb | grep 17e9

# Check EVDI
lsmod | grep evdi
ls -l /dev/dri/card*

# Check displays
xrandr --listproviders
```

## Contributing

Areas for contribution:
- Additional DisplayLink device testing
- Hardware cursor support
- H.264 compression
- Automatic EDID reading

## License

MIT License

## Disclaimer

Independent reverse-engineering project. Not affiliated with DisplayLink/Synaptics.
