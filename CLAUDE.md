# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

STARDRIVE is an open-source Linux driver for DisplayLink USB docks written in Rust. It targets the StarTech USB35DOCK (VID: 0x17e9, PID: 0x4307) and implements a reverse-engineered DisplayLink USB protocol. The driver is fully functional with multi-monitor support, hot-plug detection, dynamic resolution, and network adapter integration.

## Build Commands

### Initial Setup (First Time Only)

Build and install EVDI library and kernel module:
```bash
cd evdi_source/library && make && sudo make install
cd ../module && sudo make install && sudo modprobe evdi
cd ../..
```

Verify EVDI is loaded:
```bash
lsmod | grep evdi
```

### Building the Driver

```bash
cd displaylink-driver
cargo build --release              # Release build
cargo build                        # Debug build (with debug symbols)
```

Binary location: `target/release/displaylink-driver` or `target/debug/displaylink-driver`

### Running the Driver

Option 1 - Run as root (quick testing):
```bash
sudo ./target/release/displaylink-driver
```

Option 2 - Run as user (requires udev rules):
Create `/etc/udev/rules.d/99-displaylink.rules`:
```
SUBSYSTEM=="usb", ATTR{idVendor}=="17e9", ATTR{idProduct}=="4307", MODE="0666"
```
Then reload and run:
```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
./target/release/displaylink-driver
```

Enable verbose logging:
```bash
DISPLAYLINK_DRIVER_VERBOSE=1 ./target/release/displaylink-driver
```

### Testing

```bash
cargo test                         # Run all tests
cargo test -- --nocapture         # Show test output
cargo test test_rle_compression   # Run specific test
cargo test --release bench_       # Run performance benchmarks
```

### Code Quality

```bash
cargo fmt                         # Format code
cargo clippy                      # Lint code
```

## Architecture

The driver has a multi-layered architecture:

### Component Stack
```
DisplayLinkManager (main.rs)
  ├─ Device Scanner (hot-plug detection every 2s)
  └─ Per-Device DisplayLinkDriver instances
       ├─ EVDI Integration (FFI bindings via build.rs)
       ├─ RLE Compressor (displaylink_protocol.rs)
       ├─ USB Protocol (rusb crate)
       └─ Network Adapter (network_adapter.rs, Interface 5)
```

### Data Flow
```
X11/Wayland
    ↓ (DRM KMS)
EVDI Kernel Module (/dev/dri/cardX)
    ↓ (ioctl)
libevdi.so
    ↓ (FFI bindings)
Rust Driver (main.rs)
    ↓ (USB bulk transfers)
DisplayLink Hardware
```

### Key Modules

**main.rs** (24KB, ~800 lines):
- `DisplayLinkManager`: Multi-monitor coordinator with hot-plug support
- `DisplayLinkDriver`: Per-device state machine
- EVDI callbacks: Mode change, DPMS, update handlers
- Device lifecycle: initialization, event loop, cleanup

**displaylink_protocol.rs** (~10KB):
- `RLECompressor`: BGRA32→RGB565 conversion with Run-Length Encoding
- `CommandBuilder`: USB command formatting (register writes, bulk data)
- `DisplayMode`: Standard timing configurations (1920x1080, 1280x720, 1024x768)
- Protocol constants and format definitions

**network_adapter.rs** (~3KB):
- CDC NCM network adapter support (Interface 5)
- Non-critical initialization (display works if this fails)

**build.rs**:
- Generates FFI bindings from `evdi_source/library/evdi_lib.h` using bindgen
- Links against libevdi.so

### USB Protocol Implementation

The DisplayLink protocol (fully documented in PROTOCOL.md) uses:

**Control Transfers** (vendor-specific):
- Request 0x12: Channel initialization
- Request 0x01/0x02: Register write/read

**Bulk Transfers** (endpoint 0x01 OUT):
- Command format: `0xAF 0x20 [addr:2] [value:2]` for register writes
- Framebuffer: RLE-compressed RGB565 pixel data in 16KB chunks

**Display Mode Registers** (0x1000-0x1014):
- Width, height, timing parameters (hsync, vsync, blanking)
- Pixel clock, output enable

**Control Registers**:
- 0x1F00: Screen blanking
- 0xFF00: Sync/flush command
- 0x2000-0x2006: Damage rectangle

### EVDI Integration Details

The driver uses FFI to communicate with libevdi, which provides:
- Virtual DRM device creation (`evdi_add_device`)
- Connection management (`evdi_connect`)
- Event callbacks (mode changes, DPMS, updates, cursor, DDC/CI)
- Framebuffer access (`evdi_grab_pixels`)

Callbacks are registered in `main.rs` and forwarded to `DisplayLinkDriver` methods.

## Development Workflow

### Adding New Features

1. Understand the USB protocol (see PROTOCOL.md)
2. Implement protocol logic in `displaylink_protocol.rs` if USB-related
3. Add device-level logic in `main.rs` (DisplayLinkDriver methods)
4. Write unit tests in the module (inline `#[cfg(test)]` blocks)
5. Add integration tests in `tests/integration_test.rs`
6. Update documentation (README.md, PROTOCOL.md, or PHASE6.md as appropriate)

### Debugging USB Communication

Capture USB traffic:
```bash
sudo modprobe usbmon
sudo tcpdump -i usbmon1 -w displaylink.pcap
# Or use Wireshark with usbmon
```

Check device connection:
```bash
lsusb | grep 17e9                # Verify device is connected
ls -l /dev/dri/card*            # Check EVDI devices
xrandr                          # List displays
xrandr --listproviders          # List display providers
```

### Common Issues

**"unable to find library -levdi"**: EVDI library not installed
```bash
cd evdi_source/library && sudo make install && sudo ldconfig
```

**"Failed to add EVDI device"**: Kernel module not loaded
```bash
sudo modprobe evdi
```

**"DisplayLink device not found"**: Check connection and VID:PID
```bash
lsusb | grep 17e9
```

**Build fails with "libdrm/drm.h not found"**: Install libdrm development headers
```bash
sudo apt-get install libdrm-dev  # Ubuntu/Debian
```

## Project Phases

The project was developed in 6 phases (all complete):

1. **Phase 1**: USB device discovery
2. **Phase 2**: EVDI integration
3. **Phase 3**: Protocol analysis
4. **Phase 4**: USB infrastructure
5. **Phase 5**: Protocol implementation (RLE compression, bulk transfers, register writes)
6. **Phase 6**: Advanced features (multi-monitor, hot-plug, dynamic resolution, DPMS, network adapter, tests)

See PHASE6.md for Phase 6 feature details.

## Code Style

- Rust 2021 edition
- 4-space indentation (enforced by `cargo fmt`)
- `snake_case` for functions/variables, `CamelCase` for types, `UPPER_SNAKE_CASE` for constants
- Allow auto-generated bindings to have non-Rust naming (`#![allow(non_snake_case)]` etc.)
- Module files in `src/` match module names

## Important Constraints

- **VID:PID**: Currently hardcoded to 0x17e9:0x4307 (StarTech USB35DOCK)
- **Interfaces**: Display=0, Network=5
- **Endpoints**: Bulk OUT=0x01, Bulk IN=0x81
- **Transfer size**: 16KB max per USB bulk transfer
- **EDID**: Hardcoded 1920x1080 default in main.rs (DEFAULT_EDID constant)
- **Thread safety**: EVDI handles wrapped in `SendEvdiHandle`, USB handles in `Arc<Mutex<>>`

## Verification Commands

After making changes, verify:
```bash
# Check device detection
lsusb | grep 17e9

# Check EVDI module
lsmod | grep evdi
ls -l /dev/dri/card*

# Check display output
xrandr --listproviders
xrandr  # Should show DRI-X-Virtual-1 displays

# Test display control
xset dpms force off   # Blank screen
xset dpms force on    # Unblank screen
```

## Resources

- **BUILD.md**: Comprehensive build instructions, prerequisites, troubleshooting
- **PROTOCOL.md**: USB protocol specification, register layout, compression algorithm
- **PHASE6.md**: Advanced features documentation (multi-monitor, hot-plug, DPMS, etc.)
- **README.md**: Project overview, quick start, architecture diagram
- **AGENTS.md**: Contribution guidelines, coding style, PR requirements
