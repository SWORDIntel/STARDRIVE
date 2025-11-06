# STARDRIVE: Rust-based DisplayLink Linux Driver

An open-source Linux driver for DisplayLink USB docks, written in Rust.

## Target Device
*   **Model:** StarTech USB35DOCK
*   **Vendor ID (VID):** `0x17e9`
*   **Product ID (PID):** `0x4307`

## Current Status

### ✅ Completed (Phases 1-4)
- USB device detection and enumeration
- EVDI library integration with FFI bindings
- Virtual display creation and EDID configuration
- Event handling framework (mode changes, DPMS, cursor, DDC/CI)
- USB interface claiming and kernel driver management
- Multi-monitor support with hot-plug detection
- Dynamic resolution changing
- Power management (DPMS)
- Network adapter support (interface 5)

### 🚧 In Progress (Phase 5-6)
**Critical features needed for functional display:**
- [ ] **DisplayLink USB protocol implementation**
  - Vendor-specific USB control transfers
  - Register read/write commands
  - Device capability detection
- [ ] **Framebuffer compression/decompression**
  - BGRA32 → RGB565 color conversion
  - RLE (Run-Length Encoding) compression
  - Compression optimization
- [ ] **Actual pixel data transmission**
  - USB bulk transfer implementation
  - Chunked data transmission (16KB max)
  - Damage rectangle updates
- [ ] **Device-specific initialization**
  - Display mode configuration
  - Timing register programming
  - Screen blanking control

See [PROTOCOL.md](PROTOCOL.md) for DisplayLink protocol details.

## Quick Start

### Prerequisites
```bash
# Ubuntu/Debian
sudo apt-get install linux-headers-$(uname -r) libdrm-dev libusb-1.0-0-dev clang llvm dkms

# Fedora
sudo dnf install kernel-devel libdrm-devel libusb-devel clang llvm

# Arch Linux
sudo pacman -S linux-headers libdrm libusb clang llvm
```

### Building
```bash
# 1. Clone repository
git clone https://github.com/SWORDIntel/STARDRIVE.git
cd STARDRIVE

# 2. Build and install EVDI library
cd evdi_source/library
make && sudo make install
cd ../..

# 3. Install EVDI kernel module
cd evdi_source/module
sudo make install
sudo modprobe evdi
cd ../..

# 4. Build the driver
cd displaylink-driver
cargo build --release

# 5. Run the driver (requires Phase 5-6 completion for display output)
sudo ./target/release/displaylink-driver
```

See [BUILD.md](BUILD.md) for detailed instructions.

## Architecture

```
┌──────────────────────────────────────┐
│   DisplayLink Manager (Multi-monitor)│
├──────────────────────────────────────┤
│  DisplayLinkDriver (per device)      │
│    ├─ EVDI (Virtual Display) ✅      │
│    ├─ USB Protocol ⚠️ TODO           │
│    ├─ Framebuffer Compression ⚠️ TODO│
│    └─ Pixel Transmission ⚠️ TODO     │
└──────────────────────────────────────┘
         │              │
         ▼              ▼
  ┌───────────┐  ┌──────────┐
  │  evdi.ko  │  │ USB Core │
  └─────┬─────┘  └────┬─────┘
        │             │
        └──────┬──────┘
               ▼
        ┌──────────────┐
        │  DRM/KMS     │
        │  X11/Wayland │
        └──────────────┘
```

## Development Phases

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | USB Device Discovery | ✅ Complete |
| 2 | EVDI Integration | ✅ Complete |
| 3 | Windows Driver Analysis | ✅ Complete |
| 4 | USB Infrastructure | ✅ Complete |
| 5 | **USB Protocol & Compression** | 🚧 **In Progress** |
| 6 | Advanced Features | ✅ Complete |

## Next Steps (Phase 5)

To make the driver fully functional, the following critical components need implementation:

1. **USB Protocol Layer**
   - Reverse-engineer DisplayLink command format
   - Implement register-based communication
   - Add display mode configuration

2. **Compression Engine**
   - Implement BGRA32 → RGB565 conversion
   - Add RLE compression algorithm
   - Optimize for performance

3. **Data Transmission**
   - Implement bulk transfer protocol
   - Add chunking for large framebuffers
   - Handle USB errors and retries

4. **Display Initialization**
   - Configure timing registers
   - Set pixel clock and sync signals
   - Enable display output

## Testing

```bash
# Run tests (unit tests pass, integration requires Phase 5 completion)
cd displaylink-driver
cargo test
```

## Documentation

- **[BUILD.md](BUILD.md)** - Build and installation guide
- **[PROTOCOL.md](PROTOCOL.md)** - DisplayLink USB protocol documentation
- **[PHASE6.md](PHASE6.md)** - Advanced features documentation

## Requirements

- Linux kernel 5.0+
- Rust toolchain (edition 2021)
- EVDI kernel module and library
- libdrm development headers
- libusb 1.0

## Troubleshooting

**EVDI module not loading:**
```bash
sudo modprobe evdi
dmesg | grep evdi
```

**USB device not found:**
```bash
lsusb | grep 17e9
sudo usermod -a -G plugdev $USER
```

## Contributing

**Priority areas for contribution:**
1. USB protocol reverse engineering (Phase 5 - critical)
2. Framebuffer compression implementation
3. Bulk transfer protocol
4. Additional DisplayLink device support

## License

MIT License - See LICENSE file for details.

## Disclaimer

Independent reverse-engineering project. Not affiliated with DisplayLink/Synaptics. Based on open-source analysis. Use at your own risk.
