# STARDRIVE Submodule Integration Guide

This guide explains how to integrate STARDRIVE (the DisplayLink Linux driver) as a Git submodule into your project.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Adding STARDRIVE as a Submodule](#adding-stardrive-as-a-submodule)
- [Building the Driver](#building-the-driver)
- [Installing the Driver](#installing-the-driver)
- [Integration Workflows](#integration-workflows)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)

---

## Overview

STARDRIVE is an open-source Linux driver for DisplayLink USB docks written in Rust. It provides:

- **Multi-monitor support** with hot-plug detection
- **Dynamic resolution** and display mode switching
- **DPMS support** for power management
- **Network adapter** integration (CDC NCM)
- **Hardware acceleration** via EVDI kernel module

### Key Components

```
stardrive/
├── build.sh                          # Build script (builds everything)
├── install.sh                        # Install script (installs to system)
├── displaylink-driver/              # Rust driver source
│   ├── src/
│   ├── tests/
│   ├── Cargo.toml
│   └── build.rs                     # EVDI FFI bindings generation
├── evdi_source/                     # EVDI library & kernel module
│   ├── library/
│   └── module/
└── BUILD.md                         # Detailed build instructions

Target: StarTech USB35DOCK (VID: 0x17e9, PID: 0x4307)
```

---

## Prerequisites

### System Requirements

- **OS**: Linux (Ubuntu 20.04+ or Debian 11+ recommended)
- **Kernel**: 5.0+ (tested on 6.17)
- **Architecture**: x86_64

### Build Dependencies

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
  cargo rustc \
  gcc make \
  pkg-config libdrm-dev \
  libusb-1.0-0-dev \
  libssl-dev

# Fedora/RHEL
sudo dnf install -y \
  cargo rustc \
  gcc make \
  pkgconfig libdrm-devel \
  libusbx-devel \
  openssl-devel
```

### Runtime Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install -y \
  libevdi0 \
  libusb-1.0-0 \
  libssl3

# Fedora/RHEL
sudo dnf install -y \
  libevdi \
  libusbx \
  openssl-libs
```

---

## Adding STARDRIVE as a Submodule

### Step 1: Add the Submodule

Add STARDRIVE to your project as a Git submodule:

```bash
# Add STARDRIVE as a submodule
git submodule add https://github.com/SWORDIntel/stardrive.git drivers/stardrive

# Create a .gitmodules entry (automatically done by above command)
cat .gitmodules
```

### Step 2: Initialize and Update Submodule

```bash
# For new clones, initialize the submodule
git submodule init
git submodule update --recursive

# OR use a single command
git clone --recurse-submodules https://github.com/your-repo.git
```

### Step 3: Update Your Build System

If using a custom build system, integrate STARDRIVE's build:

#### For CMake Projects

```cmake
# CMakeLists.txt
add_subdirectory(drivers/stardrive)

# Or manually invoke STARDRIVE build
add_custom_target(build_stardrive
  COMMAND bash ${CMAKE_SOURCE_DIR}/drivers/stardrive/build.sh
  WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/drivers/stardrive
)

add_dependencies(${PROJECT_NAME} build_stardrive)
```

#### For Make Projects

```makefile
# Makefile
STARDRIVE_DIR := drivers/stardrive

.PHONY: build-stardrive install-stardrive

build-stardrive:
	cd $(STARDRIVE_DIR) && ./build.sh

install-stardrive: build-stardrive
	cd $(STARDRIVE_DIR) && sudo ./install.sh

clean-stardrive:
	cd $(STARDRIVE_DIR) && cargo clean
```

#### For Standalone Projects

Add to your `Makefile` or build script:

```bash
#!/bin/bash
# build.sh

STARDRIVE_PATH="./drivers/stardrive"

echo "Building STARDRIVE..."
cd "$STARDRIVE_PATH" || exit 1
./build.sh || exit 1

echo "Installation next. Run: sudo ./install.sh"
```

---

## Building the Driver

### Quick Build

```bash
cd drivers/stardrive
./build.sh
```

### Build Options

```bash
# Full build (library, module, driver, tests)
./build.sh

# Skip specific components
./build.sh --skip-library        # Skip EVDI library
./build.sh --skip-module         # Skip kernel module
./build.sh --skip-driver         # Skip Rust driver
./build.sh --debug               # Build in debug mode
./build.sh --verbose             # Show all build output
```

### Environment Variables

```bash
# Set build options via environment
export SKIP_LIBRARY=true
export SKIP_MODULE=true
export RELEASE_MODE=false
export VERBOSE=true
./build.sh
```

### Build Output

```
Binary location:  drivers/stardrive/displaylink-driver/target/release/displaylink-driver
Log file:         drivers/stardrive/build.log
Test results:     Included in build output
```

---

## Installing the Driver

### System Installation

```bash
cd drivers/stardrive

# Build first (if not already done)
./build.sh

# Install to system
sudo ./install.sh
```

### Installation Details

The `install.sh` script will:

1. **Copy binary** to `/usr/local/bin/displaylink-driver`
2. **Install udev rules** to `/etc/udev/rules.d/99-displaylink.rules`
3. **Create systemd service** at `/etc/systemd/system/displaylink-driver.service`
4. **Create log directory** at `/var/log/displaylink-driver.log`

### Post-Installation Setup

```bash
# 1. Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# 2. Check if device is detected
lsusb | grep 17e9

# 3. Start the driver (via systemd)
sudo systemctl start displaylink-driver

# OR start manually
sudo /usr/local/bin/displaylink-driver

# 4. Check status
sudo systemctl status displaylink-driver

# 5. View logs
tail -f /var/log/displaylink-driver.log
```

### Installation Options

```bash
# Skip systemd service
sudo ./install.sh --no-systemd

# Skip udev rules
sudo ./install.sh --no-udev

# Both
sudo ./install.sh --no-systemd --no-udev
```

---

## Integration Workflows

### Workflow 1: Docker Integration

Build STARDRIVE in a Docker image:

```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    cargo rustc gcc make pkg-config \
    libdrm-dev libusb-1.0-0-dev libssl-dev

# Copy STARDRIVE
COPY drivers/stardrive /opt/stardrive
WORKDIR /opt/stardrive

# Build
RUN ./build.sh

# Create non-root user for driver
RUN useradd -m displaylink
RUN chown -R displaylink:displaylink /opt/stardrive

# Set entry point
ENTRYPOINT ["/opt/stardrive/target/release/displaylink-driver"]
```

Build and run:

```bash
docker build -t displaylink-driver .
docker run --device /dev/bus/usb --device /dev/dri \
  --volume /var/log:/var/log \
  displaylink-driver
```

### Workflow 2: CI/CD Integration (GitHub Actions)

```yaml
name: Build STARDRIVE

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y cargo rustc gcc make \
            pkg-config libdrm-dev libusb-1.0-0-dev libssl-dev

      - name: Build STARDRIVE
        run: |
          cd drivers/stardrive
          ./build.sh --verbose

      - name: Run tests
        run: |
          cd drivers/stardrive/displaylink-driver
          cargo test --release

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: displaylink-driver
          path: drivers/stardrive/displaylink-driver/target/release/displaylink-driver
```

### Workflow 3: NixOS Flake Integration

```nix
# flake.nix
{
  description = "Project with STARDRIVE";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo rustc
          gcc gnumake pkg-config
          libdrm libusb1 openssl
        ];
      };

      packages.${system}.displaylink-driver = pkgs.callPackage
        ./drivers/stardrive/default.nix { };
    };
}
```

### Workflow 4: Ansible Deployment

```yaml
---
- name: Deploy STARDRIVE
  hosts: all
  become: yes

  tasks:
    - name: Install dependencies
      apt:
        name: "{{ packages }}"
        state: present
      vars:
        packages:
          - cargo
          - rustc
          - gcc
          - make
          - pkg-config
          - libdrm-dev
          - libusb-1.0-0-dev
          - libssl-dev

    - name: Clone repository
      git:
        repo: "https://github.com/SWORDIntel/stardrive.git"
        dest: "/opt/stardrive"
        version: main

    - name: Build driver
      shell: ./build.sh
      args:
        chdir: "/opt/stardrive"

    - name: Install driver
      shell: ./install.sh
      args:
        chdir: "/opt/stardrive"

    - name: Start service
      systemd:
        name: displaylink-driver
        state: started
        enabled: yes
```

---

## Troubleshooting

### Build Issues

#### "Unable to find library -levdi"

**Problem**: EVDI library not installed

**Solution**:
```bash
cd drivers/stardrive/evdi_source/library
make
sudo make install
sudo ldconfig
```

#### "Failed to add EVDI device"

**Problem**: Kernel module not loaded

**Solution**:
```bash
# Check if module is loaded
lsmod | grep evdi

# Load the module
sudo modprobe evdi

# OR rebuild and install
cd drivers/stardrive/evdi_source/module
make
sudo make install
sudo modprobe evdi
```

#### "cargo: command not found"

**Problem**: Rust/Cargo not installed

**Solution**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Update
rustup update
```

### Installation Issues

#### "Binary not found at /usr/local/bin/displaylink-driver"

**Problem**: Installation incomplete

**Solution**:
```bash
# Check if binary was built
ls -la drivers/stardrive/displaylink-driver/target/release/displaylink-driver

# Rebuild if missing
cd drivers/stardrive && ./build.sh

# Install again
sudo ./install.sh
```

#### "Permission denied" when running driver

**Problem**: Udev rules not loaded

**Solution**:
```bash
# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Check rules
ls -la /etc/udev/rules.d/99-displaylink.rules

# Verify device permissions
lsusb | grep 17e9
```

### Runtime Issues

#### DisplayLink device not detected

**Problem**: Device not connected or not recognized

**Solution**:
```bash
# Check USB bus
lsusb | grep 17e9

# Check dmesg for errors
dmesg | tail -20

# Check evdi module
lsmod | grep evdi

# Reload driver
sudo rmmod evdi
sudo modprobe evdi
```

#### No output from driver

**Problem**: Driver not logging

**Solution**:
```bash
# Enable verbose logging
export DISPLAYLINK_DRIVER_VERBOSE=1
sudo -E /usr/local/bin/displaylink-driver

# Check logs
tail -f /var/log/displaylink-driver.log

# Check systemd logs
sudo journalctl -u displaylink-driver -f
```

---

## Advanced Usage

### Custom Build Paths

```bash
# Build in custom location
cd /custom/path/stardrive
/path/to/stardrive/build.sh

# Install to custom prefix
PREFIX=/opt/displaylink sudo /path/to/stardrive/install.sh
```

### Development Workflow

```bash
# Clone with submodule for development
git clone --recurse-submodules https://github.com/your-repo.git
cd your-repo/drivers/stardrive

# Build in debug mode
./build.sh --debug --verbose

# Make changes
vim displaylink-driver/src/main.rs

# Rebuild
cd displaylink-driver && cargo build

# Test
cargo test
```

### Performance Tuning

```bash
# Build with optimizations
RELEASE_MODE=true ./build.sh

# Run with profiling
DISPLAYLINK_DRIVER_VERBOSE=1 \
  RUST_LOG=debug \
  sudo ./displaylink-driver/target/release/displaylink-driver

# Monitor system resources
watch -n 1 'ps aux | grep displaylink'
```

### Version Management

```bash
# Check STARDRIVE version
cd drivers/stardrive && git describe --tags

# Update to specific version
git submodule update --remote --merge
cd drivers/stardrive && git checkout v0.1.0

# Pin to current version
git submodule update --init
git add .gitmodules drivers/stardrive
git commit -m "Pin STARDRIVE to current version"
```

---

## References

- **STARDRIVE Repository**: https://github.com/SWORDIntel/stardrive
- **Build Instructions**: [BUILD.md](BUILD.md)
- **Protocol Documentation**: [PROTOCOL.md](PROTOCOL.md)
- **Phase 6 Features**: [PHASE6.md](PHASE6.md)
- **Rust Edition**: 2021
- **Target Device**: StarTech USB35DOCK (17e9:4307)

---

## Support

For issues or questions:

1. Check the [Troubleshooting](#troubleshooting) section
2. Review [BUILD.md](BUILD.md) for detailed build instructions
3. Open an issue at https://github.com/SWORDIntel/stardrive/issues
4. Review logs: `tail -f /var/log/displaylink-driver.log`

---

## License

STARDRIVE is open-source. See the LICENSE file in the repository for details.

**Last Updated**: 2025-11-19
**Compatible With**: Linux 5.0+, Rust 1.70+, Cargo 1.70+
