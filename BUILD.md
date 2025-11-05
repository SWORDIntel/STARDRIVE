# Build Instructions

This document provides comprehensive instructions for building the DisplayLink Rust driver.

## Prerequisites

### System Requirements

- Linux operating system (Ubuntu 20.04+ recommended)
- Kernel headers installed (matching your running kernel)
- Root/sudo access for EVDI kernel module installation

### Required Packages

Install the following development packages:

#### Ubuntu/Debian:
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    linux-headers-$(uname -r) \
    libdrm-dev \
    libusb-1.0-0-dev \
    pkg-config \
    clang \
    llvm \
    dkms \
    git \
    curl
```

#### Fedora/RHEL:
```bash
sudo dnf install -y \
    kernel-devel \
    libdrm-devel \
    libusbx-devel \
    pkg-config \
    clang \
    llvm \
    dkms \
    git \
    gcc \
    make
```

#### Arch Linux:
```bash
sudo pacman -S --needed \
    linux-headers \
    libdrm \
    libusb \
    pkg-config \
    clang \
    llvm \
    dkms \
    git \
    base-devel
```

### Rust Installation

Install Rust using rustup if not already installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

## Build Steps

### 1. Build EVDI Library

The EVDI (Extensible Virtual Display Interface) library provides the virtual display functionality.

```bash
cd evdi_source/library
make
sudo make install
```

This will:
- Compile `libevdi.so` shared library
- Install it to `/usr/lib/` or `/usr/local/lib/`
- Copy header files to system include path

### 2. Build and Install EVDI Kernel Module

The EVDI kernel module creates virtual DRM devices.

```bash
cd ../module
make
sudo make install
sudo depmod -a
```

Or use DKMS for automatic rebuilding on kernel updates:

```bash
cd ../..
sudo cp -r evdi_source /usr/src/evdi-1.14.11
sudo dkms add -m evdi -v 1.14.11
sudo dkms build -m evdi -v 1.14.11
sudo dkms install -m evdi -v 1.14.11
```

Load the kernel module:

```bash
sudo modprobe evdi
```

Verify it loaded:

```bash
lsmod | grep evdi
```

### 3. Build DisplayLink Rust Driver

```bash
cd displaylink-driver
cargo build --release
```

The compiled binary will be at: `target/release/displaylink-driver`

## Running the Driver

### Required Permissions

The driver needs access to USB devices. You can either:

**Option 1: Run as root (not recommended for production)**
```bash
sudo ./target/release/displaylink-driver
```

**Option 2: Add udev rules (recommended)**

Create `/etc/udev/rules.d/99-displaylink.rules`:
```
# StarTech USB35DOCK
SUBSYSTEM=="usb", ATTR{idVendor}=="17e9", ATTR{idProduct}=="4307", MODE="0666"
```

Reload udev rules:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Then run as normal user:
```bash
./target/release/displaylink-driver
```

### Verification

1. Connect your DisplayLink device (StarTech USB35DOCK)
2. Run the driver:
   ```bash
   ./target/release/displaylink-driver
   ```
3. You should see output like:
   ```
   DisplayLink Rust Driver v0.1.0
   ========================================
   EVDI library version: 1.14.11
   USB context initialized.
   DisplayLink device found!
     Bus: 2, Address: 5
     VID: 0x17E9, PID: 0x4307
   USB device opened successfully.
   Created EVDI device: /dev/dri/card1
   Connecting to EVDI with default EDID...
   Claiming interface 0
   DisplayLink device initialized successfully.
   ```

4. Check for new display device:
   ```bash
   ls -l /dev/dri/
   xrandr  # Should show new virtual display
   ```

## Troubleshooting

### "unable to find library -levdi"

The EVDI library is not installed or not in the library path.

**Solution:**
```bash
cd evdi_source/library
sudo make install
sudo ldconfig
```

### "Failed to add EVDI device"

The EVDI kernel module is not loaded.

**Solution:**
```bash
sudo modprobe evdi
```

### "Permission denied" opening USB device

USB device permissions are not configured.

**Solution:**
- Add udev rules (see "Required Permissions" above)
- Or run with sudo (not recommended)

### "DisplayLink device not found"

The device is not connected or not recognized.

**Solution:**
- Verify device is connected: `lsusb | grep 17e9`
- Check USB cable and port
- Try different USB port
- Verify VID:PID matches your device in `src/main.rs`

### Build fails with "libdrm/drm.h: No such file or directory"

libdrm development headers are not installed.

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install libdrm-dev

# Fedora
sudo dnf install libdrm-devel

# Arch
sudo pacman -S libdrm
```

### "bindgen" fails

Clang/LLVM is not installed or not found.

**Solution:**
```bash
# Ubuntu/Debian
sudo apt-get install clang llvm

# Fedora
sudo dnf install clang llvm

# Arch
sudo pacman -S clang llvm
```

## Development

### Debug Build

For development with debug symbols:
```bash
cargo build
./target/debug/displaylink-driver
```

### Enable Verbose Logging

Set Rust log level:
```bash
RUST_LOG=debug ./target/release/displaylink-driver
```

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## Current Limitations

### DisplayLink USB Protocol

The DisplayLink USB protocol is **proprietary and undocumented**. The current implementation includes:

✅ **Implemented:**
- USB device detection and enumeration
- Device initialization (interface claiming, kernel driver detachment)
- EVDI virtual display creation and connection
- Event handling framework (DPMS, mode changes, cursor, DDC/CI)
- Framebuffer management and registration
- EDID configuration (default 1920x1080)

⚠️ **Not Implemented (Requires Reverse Engineering):**
- DisplayLink compression algorithm
- USB bulk transfer protocol for framebuffer data
- Device-specific initialization sequences
- Firmware upload (if required)
- Network adapter functionality (interface 5)

### Reverse Engineering Requirements

To complete the USB protocol implementation:

1. **USB Packet Capture:**
   ```bash
   # Enable USB monitoring
   sudo modprobe usbmon

   # Capture USB traffic
   sudo tcpdump -i usbmon1 -w displaylink.pcap

   # Or use Wireshark with usbmon
   ```

2. **strace Official Driver:**
   ```bash
   sudo strace -f -e trace=ioctl,read,write DisplayLinkManager
   ```

3. **Binary Analysis:**
   - Reverse engineer `libdlm.so` from official driver
   - Analyze USB control/bulk transfer sequences
   - Document compression format

## Installation

### System-Wide Installation

```bash
sudo cp target/release/displaylink-driver /usr/local/bin/
```

### Systemd Service

Create `/etc/systemd/system/displaylink.service`:

```ini
[Unit]
Description=DisplayLink USB Graphics Driver
After=multi-user.target

[Service]
Type=simple
ExecStart=/usr/local/bin/displaylink-driver
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable displaylink
sudo systemctl start displaylink
```

## Architecture

```
┌─────────────────────────────────────┐
│     DisplayLink Rust Driver         │
│                                     │
│  ┌──────────────┐  ┌────────────┐  │
│  │ EVDI FFI     │  │ USB (rusb) │  │
│  │ Bindings     │  │ Interface  │  │
│  └──────┬───────┘  └─────┬──────┘  │
└─────────┼──────────────────┼─────────┘
          │                  │
          ▼                  ▼
┌─────────────────┐  ┌──────────────┐
│  libevdi.so     │  │  libusb      │
│  (User Space)   │  │  (User Space)│
└────────┬────────┘  └──────┬───────┘
         │                  │
         ▼                  ▼
┌─────────────────┐  ┌──────────────┐
│  evdi.ko        │  │  USB Stack   │
│  (Kernel Module)│  │  (Kernel)    │
└────────┬────────┘  └──────┬───────┘
         │                  │
         ▼                  ▼
    ┌────────────────────────┐
    │    DRM Subsystem       │
    │    (Linux Kernel)      │
    └───────────┬────────────┘
                │
                ▼
         ┌──────────────┐
         │  X11/Wayland │
         │  Compositor  │
         └──────────────┘
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

This driver is licensed under LGPL-2.1 to maintain compatibility with the EVDI library.

## Support

For issues and questions:
- GitHub Issues: https://github.com/yourusername/STARDRIVE/issues
- Documentation: See README.md
