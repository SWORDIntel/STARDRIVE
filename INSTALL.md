# DisplayLink Driver Installation Guide

Complete guide to building and installing the STARDRIVE DisplayLink driver.

## Table of Contents

- [Quick Start](#quick-start)
- [System Requirements](#system-requirements)
- [Building from Source](#building-from-source)
- [Installing the Driver](#installing-the-driver)
- [Post-Installation Configuration](#post-installation-configuration)
- [Testing the Installation](#testing-the-installation)
- [Troubleshooting](#troubleshooting)
- [Uninstallation](#uninstallation)

---

## Quick Start

For the impatient, here are the essential commands:

```bash
# 1. Clone the repository (or use as git submodule)
git clone https://github.com/SWORDIntel/stardrive.git
cd stardrive

# 2. Build everything
./build.sh

# 3. Install to system
sudo ./install.sh

# 4. Setup udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# 5. Start the driver
sudo systemctl start displaylink-driver

# 6. Verify it's running
sudo systemctl status displaylink-driver
```

---

## System Requirements

### Hardware

- **DisplayLink Device**: StarTech USB35DOCK or compatible (VID: 0x17e9)
- **CPU**: x86_64 processor
- **RAM**: 512 MB minimum (1 GB recommended)
- **Disk**: 500 MB free space for build artifacts

### Operating System

| OS | Version | Status |
|---|---|---|
| Ubuntu | 20.04 LTS+ | ✅ Tested |
| Debian | 11+ | ✅ Tested |
| Fedora | 35+ | ✅ Should work |
| Red Hat | 8+ | ✅ Should work |
| Arch | Latest | ✅ Should work |

### Kernel

- **Minimum**: Linux 5.0+
- **Recommended**: 6.0+
- **Tested**: 6.17

Check your kernel version:
```bash
uname -r
```

### Architecture

- **x86_64**: ✅ Fully supported
- **ARM64**: ❌ Not tested
- **ARM32**: ❌ Not supported

---

## Building from Source

### Step 1: Install Build Dependencies

#### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  cargo \
  rustc \
  gcc \
  make \
  pkg-config \
  libdrm-dev \
  libusb-1.0-0-dev \
  libssl-dev \
  git
```

#### Fedora/RHEL

```bash
sudo dnf install -y \
  gcc \
  make \
  cargo \
  rustc \
  pkg-config \
  libdrm-devel \
  libusbx-devel \
  openssl-devel \
  git
```

#### Arch Linux

```bash
sudo pacman -S --noconfirm \
  base-devel \
  rust \
  cargo \
  libdrm \
  libusb \
  openssl
```

### Step 2: Clone the Repository

```bash
# Clone with all submodules
git clone --recurse-submodules https://github.com/SWORDIntel/stardrive.git
cd stardrive

# OR if already cloned without submodules
git submodule update --init --recursive
```

### Step 3: Build the Driver

Run the build script:

```bash
./build.sh
```

The script will:

1. Build EVDI library
2. Install EVDI library to system
3. Build EVDI kernel module
4. Install EVDI kernel module
5. Build Rust DisplayLink driver
6. Run unit and integration tests

**Build takes approximately 2-5 minutes on modern hardware.**

#### Build Options

```bash
# Skip specific components (for faster builds)
./build.sh --skip-library        # Skip EVDI library build
./build.sh --skip-module         # Skip kernel module (if already installed)
./build.sh --skip-driver         # Skip Rust driver only
./build.sh --debug               # Build in debug mode
./build.sh --verbose             # Show all compiler output

# Examples
./build.sh --skip-library --skip-module  # Only build driver
./build.sh --debug --verbose             # Debug with output
```

#### Verify Build Success

```bash
# Check binary was created
ls -lh displaylink-driver/target/release/displaylink-driver

# Run tests
cd displaylink-driver
cargo test --release
cd ..
```

---

## Installing the Driver

### System-Wide Installation

```bash
# Install everything (binary, udev rules, systemd service)
sudo ./install.sh
```

**Installation locations:**

| Component | Location |
|---|---|
| Binary | `/usr/local/bin/displaylink-driver` |
| Udev Rules | `/etc/udev/rules.d/99-displaylink.rules` |
| Systemd Service | `/etc/systemd/system/displaylink-driver.service` |
| Logs | `/var/log/displaylink-driver.log` |
| EVDI Library | `/usr/local/lib/libevdi.so` |

### Selective Installation

```bash
# Skip systemd service (use manual startup only)
sudo ./install.sh --no-systemd

# Skip udev rules (run as root only)
sudo ./install.sh --no-udev

# Skip both (minimal installation)
sudo ./install.sh --no-systemd --no-udev
```

---

## Post-Installation Configuration

### 1. Reload Udev Rules

```bash
# Reload udev rules
sudo udevadm control --reload-rules

# Trigger udev events
sudo udevadm trigger

# Verify rules are loaded
cat /etc/udev/rules.d/99-displaylink.rules
```

### 2. Verify EVDI Module

```bash
# Check if EVDI module is loaded
lsmod | grep evdi

# Output should show:
# evdi                   98304  0
# drm_kms_helper        258048  8 evdi,...

# If not loaded, load it manually
sudo modprobe evdi

# Make it permanent (auto-load on boot)
echo "evdi" | sudo tee /etc/modules-load.d/evdi.conf
```

### 3. Check DisplayLink Device

```bash
# List USB devices
lsusb | grep 17e9

# Should output something like:
# Bus 004 Device 003: ID 17e9:4307 DisplayLink USB 3.0 Dual Video Dock

# Check device permissions
ls -l /dev/bus/usb/*/

# Verify /dev/dri devices exist
ls -l /dev/dri/card*
```

### 4. Configure User Access (Optional)

To run the driver without sudo, add your user to the plugdev and video groups:

```bash
# Add user to groups
sudo usermod -aG plugdev,video $USER

# Apply group changes (without logout/login)
newgrp plugdev
newgrp video

# Verify group membership
groups $USER
```

---

## Testing the Installation

### Test 1: Check Binary

```bash
# Verify binary is executable
test -x /usr/local/bin/displaylink-driver && echo "Binary OK" || echo "Binary NOT OK"

# Check binary size
ls -lh /usr/local/bin/displaylink-driver
```

### Test 2: Verify Dependencies

```bash
# Check EVDI library
ldconfig -p | grep evdi
# Should output: libevdi.so (...)

# Check libusb
ldconfig -p | grep libusb
# Should output: libusb-1.0.so (...)

# Check OpenSSL
ldconfig -p | grep ssl
# Should output: libssl.so.3 (...)
```

### Test 3: Start via Systemd

```bash
# Start the driver
sudo systemctl start displaylink-driver

# Check status
sudo systemctl status displaylink-driver

# Expected output:
# displaylink-driver.service - DisplayLink USB Driver Service
#      Loaded: loaded (/etc/systemd/system/displaylink-driver.service; enabled; ...)
#      Active: active (running) since ...

# View logs
sudo journalctl -u displaylink-driver -n 50

# Follow logs in real-time
sudo journalctl -u displaylink-driver -f
```

### Test 4: Manual Start (Debug)

```bash
# Run with verbose logging
export DISPLAYLINK_DRIVER_VERBOSE=1
sudo -E /usr/local/bin/displaylink-driver

# Look for output like:
# [timestamp] Initializing DisplayLink device...
# [timestamp] Device initialized successfully

# Press Ctrl+C to stop
```

### Test 5: Check Display Output

```bash
# List display providers
xrandr --listproviders

# List displays
xrandr

# Set resolution (if detected)
xrandr --output [name] --mode 1920x1080 --rate 60
```

### Test 6: Run Tests

```bash
# Run driver unit tests
cd displaylink-driver
cargo test --release

# Expected output:
# test result: ok. 42 passed; 0 failed

# Run integration tests
cargo test --release --test '*'

# Expected output:
# test result: ok. 11 passed; 0 failed
```

---

## Troubleshooting

### Build Failures

#### "cargo: command not found"

**Solution**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Update to latest
rustup update
```

#### "libdrm/drm.h: No such file or directory"

**Solution**:
```bash
# Install libdrm development headers
sudo apt-get install libdrm-dev          # Ubuntu/Debian
sudo dnf install libdrm-devel            # Fedora
```

#### "unable to find library -levdi"

**Solution**:
```bash
# Rebuild EVDI library
cd evdi_source/library && make && sudo make install
sudo ldconfig
cd ../..

# Rebuild driver
cd displaylink-driver && cargo build --release
```

#### Build hangs or is very slow

**Solution**:
```bash
# Build in debug mode (faster compilation)
./build.sh --debug

# Or skip some components
./build.sh --skip-library --skip-module --skip-driver

# Check system resources
free -h        # Memory
df -h          # Disk space
top -b -n1     # CPU usage
```

### Installation Failures

#### "Permission denied" during install

**Solution**:
```bash
# Must run as root
sudo ./install.sh

# OR if using sudo without password
sudo -E ./install.sh
```

#### "Failed to install binary"

**Solution**:
```bash
# Check /usr/local/bin is writable
ls -ld /usr/local/bin
sudo chmod 755 /usr/local/bin

# Try installation again
sudo ./install.sh
```

### Runtime Issues

#### Driver won't start

**Solution**:
```bash
# Check the binary exists and is executable
sudo -l /usr/local/bin/displaylink-driver

# Run with debugging
sudo DISPLAYLINK_DRIVER_VERBOSE=1 /usr/local/bin/displaylink-driver

# Check systemd service
sudo systemctl status displaylink-driver
sudo journalctl -u displaylink-driver -n 100

# Check hardware
lsusb | grep 17e9
lsmod | grep evdi
```

#### Device not detected

**Solution**:
```bash
# Check USB connection
lsusb | grep 17e9
# Should show: ID 17e9:4307 DisplayLink...

# If not detected, try USB port
# If detected, check dmesg
dmesg | tail -50 | grep -i "displaylink\|evdi\|usb"

# Check kernel module
lsmod | grep evdi

# If not loaded
sudo modprobe evdi
```

#### No display output

**Solution**:
```bash
# Verify EVDI created virtual display
ls /dev/dri/card*

# Check X/Wayland
echo $DISPLAY      # For X11
echo $WAYLAND_DISPLAY  # For Wayland

# Check xrandr
xrandr --listproviders
xrandr -d :0       # For X11

# Enable verbose logging and check output
DISPLAYLINK_DRIVER_VERBOSE=1 sudo /usr/local/bin/displaylink-driver
```

#### "Cannot open device: -4" (EVDI device error)

**Solution**:
```bash
# EVDI module needs reloading
sudo rmmod evdi
sudo modprobe evdi

# Or restart the driver
sudo systemctl restart displaylink-driver
```

### Log Analysis

```bash
# View recent logs
tail -100 /var/log/displaylink-driver.log

# Search for errors
grep "error\|failed\|ERROR" /var/log/displaylink-driver.log

# Monitor in real-time
tail -f /var/log/displaylink-driver.log

# View systemd logs
journalctl -u displaylink-driver -n 100

# With timestamps
journalctl -u displaylink-driver --no-pager -n 100 -o short-iso
```

---

## Uninstallation

### Remove System Installation

```bash
# Stop the service
sudo systemctl stop displaylink-driver

# Disable from boot
sudo systemctl disable displaylink-driver

# Remove files
sudo rm -f /usr/local/bin/displaylink-driver
sudo rm -f /etc/systemd/system/displaylink-driver.service
sudo rm -f /etc/udev/rules.d/99-displaylink.rules

# Reload systemd
sudo systemctl daemon-reload

# Reload udev
sudo udevadm control --reload-rules
```

### Remove Build Artifacts

```bash
cd stardrive

# Clean Rust build
cd displaylink-driver && cargo clean && cd ..

# Clean EVDI build (optional - needed for other projects)
# cd evdi_source/library && make clean && cd ../module && make clean
```

### Remove EVDI Library (Complete Uninstall)

```bash
# WARNING: Only if you don't need EVDI for other projects

# Unload kernel module
sudo rmmod evdi

# Uninstall library
cd evdi_source/library
sudo make uninstall

# Update library cache
sudo ldconfig
```

---

## Configuration

### Environment Variables

```bash
# Enable verbose logging
export DISPLAYLINK_DRIVER_VERBOSE=1

# Set log file location (if not using systemd)
export DISPLAYLINK_DRIVER_LOG=/var/log/displaylink-driver.log

# Set library path (if needed)
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Rust logging (for debugging)
export RUST_LOG=debug
```

### Systemd Configuration

Edit `/etc/systemd/system/displaylink-driver.service` to customize:

```ini
[Service]
# Change log level
Environment="DISPLAYLINK_DRIVER_VERBOSE=1"

# Change restart policy
Restart=always
RestartSec=10

# Change user/group
User=displaylink
Group=displaylink

# Reload after changes
sudo systemctl daemon-reload
sudo systemctl restart displaylink-driver
```

---

## Support

- **Repository**: https://github.com/SWORDIntel/stardrive
- **Issues**: https://github.com/SWORDIntel/stardrive/issues
- **Documentation**: See BUILD.md, PROTOCOL.md, PHASE6.md

---

## Additional Resources

- [Build Instructions](BUILD.md)
- [Submodule Integration Guide](SUBMODULE_INTEGRATION.md)
- [Protocol Documentation](PROTOCOL.md)
- [Phase 6 Features](PHASE6.md)

---

**Last Updated**: 2025-11-19
**Tested On**: Ubuntu 22.04, Debian 12, Kernel 6.17+
**Rust Version**: 1.70+
