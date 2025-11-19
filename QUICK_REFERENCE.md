# STARDRIVE Quick Reference

Quick command reference for building, installing, and using the DisplayLink driver.

## Table of Contents

- [Build Commands](#build-commands)
- [Install Commands](#install-commands)
- [Service Management](#service-management)
- [Debugging](#debugging)
- [Troubleshooting](#troubleshooting)

---

## Build Commands

### Full Build (EVDI + Kernel Module + Driver)
```bash
./build.sh
```

### Fast Build (Skip Library & Module)
```bash
SKIP_LIBRARY=true SKIP_MODULE=true ./build.sh
```

### Debug Build
```bash
./build.sh --debug
```

### Build with Output
```bash
./build.sh --verbose
```

### Skip Specific Components
```bash
./build.sh --skip-library
./build.sh --skip-module
./build.sh --skip-driver
```

### Show Help
```bash
./build.sh --help
```

### Check Build Log
```bash
tail -f build.log
```

---

## Install Commands

### Standard Installation
```bash
sudo ./install.sh
```

### Skip Udev Rules
```bash
sudo ./install.sh --no-udev
```

### Skip Systemd Service
```bash
sudo ./install.sh --no-systemd
```

### Skip Both (Minimal)
```bash
sudo ./install.sh --no-udev --no-systemd
```

### Show Help
```bash
./install.sh --help
```

---

## Service Management

### Start Service
```bash
sudo systemctl start displaylink-driver
```

### Stop Service
```bash
sudo systemctl stop displaylink-driver
```

### Restart Service
```bash
sudo systemctl restart displaylink-driver
```

### Enable on Boot
```bash
sudo systemctl enable displaylink-driver
```

### Disable on Boot
```bash
sudo systemctl disable displaylink-driver
```

### Check Status
```bash
sudo systemctl status displaylink-driver
```

### View Service Logs
```bash
sudo journalctl -u displaylink-driver -f
```

### View All Service Logs
```bash
sudo journalctl -u displaylink-driver -n 100
```

---

## Udev Management

### Reload Udev Rules
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Test Device Recognition
```bash
udevadm test /sys/devices/pci0000:00/.../usb4/4-1
```

### Check Device Permissions
```bash
lsusb | grep 17e9
ls -la /dev/dri/card*
```

---

## Debugging

### Run Driver Manually
```bash
sudo /usr/local/bin/displaylink-driver
```

### Run with Verbose Logging
```bash
DISPLAYLINK_DRIVER_VERBOSE=1 sudo /usr/local/bin/displaylink-driver
```

### View Log File
```bash
tail -f /var/log/displaylink-driver.log
```

### Check Device Detection
```bash
lsusb | grep 17e9
```

### Check EVDI Module
```bash
lsmod | grep evdi
```

### Check Display Configuration
```bash
xrandr
xrandr --listproviders
```

---

## Common Workflows

### Fresh Installation (All Steps)
```bash
cd stardrive
./build.sh
sudo ./install.sh
sudo udevadm control --reload-rules && sudo udevadm trigger
sudo systemctl start displaylink-driver
sudo systemctl status displaylink-driver
```

### Development Iteration
```bash
# Initial setup
./build.sh
sudo ./install.sh

# Edit code
vim displaylink-driver/src/main.rs

# Rebuild quickly
SKIP_LIBRARY=true SKIP_MODULE=true ./build.sh

# Test
sudo systemctl restart displaylink-driver
tail -f /var/log/displaylink-driver.log
```

### Run Tests
```bash
cd displaylink-driver
cargo test --release
```

### Build in Debug Mode
```bash
./build.sh --debug
cd displaylink-driver
cargo build  # Debug binary at target/debug/
```

### Clean Build
```bash
cd displaylink-driver
cargo clean
cd ..
./build.sh
```

---

## Troubleshooting Quick Fixes

### Driver Won't Start
```bash
# Check if binary exists
ls -la /usr/local/bin/displaylink-driver

# Check systemd errors
sudo journalctl -u displaylink-driver -n 50

# Check device detection
lsusb | grep 17e9

# Check EVDI module
lsmod | grep evdi
```

### Device Not Detected
```bash
# List USB devices
lsusb | grep 17e9

# Check kernel messages
dmesg | tail -20

# Reload EVDI module
sudo rmmod evdi
sudo modprobe evdi
```

### Udev Rules Not Working
```bash
# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Verify rules exist
cat /etc/udev/rules.d/99-displaylink.rules

# Test rules
udevadm test /sys/devices/.../4-1
```

### Permission Denied Errors
```bash
# Check user groups
groups $USER

# Add to required groups
sudo usermod -aG plugdev,video $USER

# Log out and back in
exit
# login again

# Verify groups
groups $USER
```

---

## Environment Variables

### Build Variables
```bash
# Skip library build
export SKIP_LIBRARY=true

# Skip kernel module build
export SKIP_MODULE=true

# Skip Rust driver build
export SKIP_DRIVER=true

# Debug mode
export RELEASE_MODE=false

# Show all output
export VERBOSE=true
```

### Runtime Variables
```bash
# Enable verbose logging
export DISPLAYLINK_DRIVER_VERBOSE=1

# Set library path
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Rust debugging
export RUST_LOG=debug
```

---

## File Locations

### Source Files
```
stardrive/
├── build.sh                      # Build script
├── install.sh                    # Install script
├── displaylink-driver/           # Driver source
├── evdi_source/                  # EVDI library/module
├── INSTALL.md                    # Installation guide
└── SUBMODULE_INTEGRATION.md      # Integration guide
```

### Installation Paths
```
/usr/local/bin/displaylink-driver           # Binary
/etc/systemd/system/displaylink-driver.service  # Service
/etc/udev/rules.d/99-displaylink.rules      # Udev rules
/var/log/displaylink-driver.log             # Log file
/usr/local/lib/libevdi.so                   # EVDI library
```

---

## Performance Tips

### Fast Rebuild During Development
```bash
SKIP_LIBRARY=true SKIP_MODULE=true ./build.sh
```

### Use Release Mode for Production
```bash
# Already default, but explicit:
export RELEASE_MODE=true
./build.sh
```

### Monitor System Resources
```bash
free -h          # Memory usage
df -h            # Disk usage
top -b -n 1      # CPU/process info
```

---

## Additional Resources

- [INSTALL.md](INSTALL.md) - Detailed installation guide
- [SUBMODULE_INTEGRATION.md](SUBMODULE_INTEGRATION.md) - Integration guide
- [BUILD.md](BUILD.md) - Build system details
- [PROTOCOL.md](PROTOCOL.md) - USB protocol specification
- [PHASE6.md](PHASE6.md) - Feature documentation

---

## Getting Help

### View Script Help
```bash
./build.sh --help
./install.sh --help
```

### Check Logs
```bash
tail -100 build.log
tail -100 /var/log/displaylink-driver.log
sudo journalctl -u displaylink-driver -n 100
```

### Run Tests
```bash
cd displaylink-driver
cargo test --release -- --nocapture
```

### Report Issues
1. Check [INSTALL.md](INSTALL.md) troubleshooting section
2. Review logs with verbose logging enabled
3. Check GitHub issues: https://github.com/SWORDIntel/stardrive/issues

---

## Quick Command Reference

| Task | Command |
|------|---------|
| Build | `./build.sh` |
| Install | `sudo ./install.sh` |
| Test | `cd displaylink-driver && cargo test --release` |
| Start | `sudo systemctl start displaylink-driver` |
| Stop | `sudo systemctl stop displaylink-driver` |
| Status | `sudo systemctl status displaylink-driver` |
| Logs | `tail -f /var/log/displaylink-driver.log` |
| Check Device | `lsusb \| grep 17e9` |
| Check Module | `lsmod \| grep evdi` |
| Reload Udev | `sudo udevadm control --reload-rules && sudo udevadm trigger` |
| Help (Build) | `./build.sh --help` |
| Help (Install) | `./install.sh --help` |

---

**Last Updated**: 2025-11-19
**For Complete Guide**: See [INSTALL.md](INSTALL.md)
