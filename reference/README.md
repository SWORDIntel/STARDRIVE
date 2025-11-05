# Reference Materials

This directory contains reference materials used during the development and reverse engineering of the STARDRIVE DisplayLink driver.

## Contents

### DisplayLink USB Graphics Software for Ubuntu 6.2-EXE
Official DisplayLink driver package for Linux (version 6.2.0-30).

**Contents:**
- `DisplayLinkManager` binaries for multiple architectures:
  - x64-ubuntu-1604
  - x86-ubuntu-1604
  - aarch64-linux-gnu
  - arm-linux-gnueabihf
- EVDI kernel module source (`evdi.tar.gz`)
- Firmware packages (`.spkg` files):
  - `ella-dock-release.spkg`
  - `firefly-monitor-release.spkg`
  - `navarro-dock-release.spkg`
  - `ridge-dock-release.spkg`
- Installation scripts

**Usage:**
These files were used for:
- Analyzing the official driver's USB communication patterns
- Understanding the DisplayLinkManager binary behavior
- Reference for EVDI integration
- Firmware package format analysis

### displaylink_strace.log
System call trace of the DisplayLinkManager daemon during startup and operation.

**Size:** ~1.5 MB
**Generated with:** `strace -f DisplayLinkManager`

**Contents:**
- Library loading sequences
- USB device enumeration
- File system operations
- Configuration file access
- Network operations

**Usage:**
Used for understanding:
- Runtime dependencies
- USB device communication patterns
- Configuration file locations
- Library call sequences

## Notes

These reference files are **not required** for building or running the STARDRIVE driver. They were used during the reverse engineering and development process and are kept for:

1. **Documentation purposes** - showing the analysis process
2. **Future reference** - for implementing additional features
3. **Comparison** - validating our implementation against the official driver
4. **Education** - helping others understand the reverse engineering process

## Official Driver vs STARDRIVE

| Feature | Official Driver | STARDRIVE |
|---------|----------------|-----------|
| Language | C/C++ | Rust |
| License | Proprietary | Open Source (LGPL-2.1) |
| EVDI | Required | Required |
| USB Protocol | Proprietary | Reverse-engineered |
| Dependencies | Many libraries | Minimal (libusb, libevdi) |
| Size | ~62 MB | ~185 KB (compiled) |
| Kernel Version | Specific versions | Generic (4.15+) |

## See Also

- [PROTOCOL.md](../PROTOCOL.md) - Complete protocol documentation
- [BUILD.md](../BUILD.md) - Build instructions
- [README.md](../README.md) - Project overview
