# STARDRIVE: DisplayLink USB Driver (Rust)

Open-source Linux driver for DisplayLink USB docks, written in Rust.

Do not ever do this,it was awful i was like lets harden all the thigns and didnt stop to think WHY GOD DID I DO THIS ITS PROPRIETRY...anyway it works for...every purpose so far,open issues,im sure theres many.

## Target Device
**StarTech USB35DOCK** - VID: `0x17e9`, PID: `0x4307`

## Status: ✅ COMPLETE (Optimization & Architecture) - "On My Machine"

Full-featured driver with reverse-engineered DisplayLink USB protocol, now highly optimized.

### Implemented Features
- ✅ **USB Protocol** - Vendor control transfers, register writes, bulk transfers
- ✅ **Framebuffer** - BGRA32→RGB565 conversion, RLE compression, pixel transmission
- ✅ **Display** - EVDI integration, EDID config, mode setting, timing generation
- ✅ **Multi-monitor** - Unlimited devices, hot-plug detection, concurrent operation
- ✅ **Power** - Full DPMS support (ON/STANDBY/SUSPEND/OFF)
- ✅ **High Performance** - 60 FPS support, ~16ms frame interval
- ✅ **Advanced Color** - Ordered Dithering (Bayer 2x2) for smooth gradients
- ✅ **Networking** - Basic CDC NCM support structure

### Performance
- Frame Rate: **60 fps** @ 1920x1080 (previously capped at 30)
- Compression: Optimized RLE with Raw Run Aggregation
- Latency: <16ms (compression + USB transfer)
- Optimization: Buffer pooling, pre-allocation, efficient hot-plug polling (1s)

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
DisplayLink Manager (src/manager.rs)
  ├─ Device Scanner (1s polling)
  └─ Per-Device Drivers (src/driver.rs)
       ├─ EVDI Interface (src/evdi.rs)
       ├─ Protocol Engine (src/displaylink_protocol.rs)
       │    ├─ RLE Compressor (w/ Dithering)
       │    └─ Command Builder
       └─ USB Transport (rusb)
```

## Development Phases

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | USB Device Discovery | ✅ |
| 2 | EVDI Integration | ✅ |
| 3 | Protocol Analysis | ✅ |
| 4 | USB Infrastructure | ✅ |
| 5 | Protocol Implementation | ✅ |
| 6 | Advanced Features | ✅ |
| 7 | **Optimization & Architecture** | ✅ |

**Phase 7 Details:**
- Modular code structure (Manager/Driver separation)
- Safe FFI encapsulation
- 60 FPS unlock (10ms sleep interval)
- Bayer 2x2 Ordered Dithering
- Fixed protocol opcode collisions (0xAF/0x20)
- Instant hot-plug detection

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
