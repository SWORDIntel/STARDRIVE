#!/usr/bin/env bash
#
# DisplayLink Driver Installation Script
# Installs the compiled driver and sets up udev rules
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}"
INSTALL_LOG="${ROOT_DIR}/install.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Installation paths
BINARY_SOURCE="${ROOT_DIR}/displaylink-driver/target/release/displaylink-driver"
BINARY_DEST="/usr/local/bin/displaylink-driver"
UDEV_RULES_SOURCE="${SCRIPT_DIR}/99-displaylink.rules"
UDEV_RULES_DEST="/etc/udev/rules.d/99-displaylink.rules"
SYSTEMD_SERVICE_SOURCE="${SCRIPT_DIR}/displaylink-driver.service"
SYSTEMD_SERVICE_DEST="/etc/systemd/system/displaylink-driver.service"

# Helper functions
log() {
  echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $*" | tee -a "$INSTALL_LOG"
}

error() {
  echo -e "${RED}[ERROR]${NC} $*" | tee -a "$INSTALL_LOG" >&2
  exit 1
}

success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*" | tee -a "$INSTALL_LOG"
}

warn() {
  echo -e "${YELLOW}[WARN]${NC} $*" | tee -a "$INSTALL_LOG"
}

usage() {
  cat <<'EOF'
Usage: ./install.sh [OPTIONS]

Install the DisplayLink driver to the system.

This script must be run as root or with sudo.

OPTIONS:
  --no-udev         Skip udev rules installation
  --no-systemd      Skip systemd service installation
  --help            Show this help message

EXAMPLES:
  sudo ./install.sh                   # Full installation
  sudo ./install.sh --no-systemd      # Skip systemd setup
  sudo ./install.sh --no-udev         # Skip udev rules

After installation:
  1. Reload udev rules:
     sudo udevadm control --reload-rules
     sudo udevadm trigger

  2. Start the driver:
     sudo systemctl start displaylink-driver

     OR manually:
     sudo ./target/release/displaylink-driver

  3. Check status:
     sudo systemctl status displaylink-driver
     tail -f /var/log/displaylink-driver.log

EOF
}

# Parse arguments
INSTALL_UDEV=true
INSTALL_SYSTEMD=true

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-udev) INSTALL_UDEV=false ;;
    --no-systemd) INSTALL_SYSTEMD=false ;;
    --help) usage; exit 0 ;;
    *)
      error "Unknown option: $1"
      ;;
  esac
  shift
done

# Check if running as root
check_root() {
  if [[ $EUID -ne 0 ]]; then
    error "This script must be run as root or with sudo"
  fi
}

# Verify binary exists
verify_binary() {
  log "Verifying binary..."

  if [[ ! -f "$BINARY_SOURCE" ]]; then
    error "Binary not found: $BINARY_SOURCE"
  fi

  if [[ ! -x "$BINARY_SOURCE" ]]; then
    error "Binary is not executable: $BINARY_SOURCE"
  fi

  log "Binary verified: $BINARY_SOURCE"
}

# Install binary
install_binary() {
  log "Installing binary to $BINARY_DEST..."

  if cp "$BINARY_SOURCE" "$BINARY_DEST" 2>/dev/null; then
    chmod 755 "$BINARY_DEST"
    success "Binary installed"
  else
    error "Failed to install binary"
  fi
}

# Create udev rules
create_udev_rules() {
  if [[ "$INSTALL_UDEV" != "true" ]]; then
    warn "Skipping udev rules installation"
    return 0
  fi

  log "Creating udev rules..."

  # Create temporary udev rules file
  cat > /tmp/99-displaylink.rules <<'EOF'
# DisplayLink USB Device Rules
# Allow non-root access to DisplayLink USB devices

# StarTech USB35DOCK (17e9:4307)
SUBSYSTEM=="usb", ATTR{idVendor}=="17e9", ATTR{idProduct}=="4307", MODE="0666"

# Generic DisplayLink devices
SUBSYSTEM=="usb", ATTR{idVendor}=="17e9", MODE="0666"

# EVDI virtual display device
SUBSYSTEM=="drm", KERNEL=="card*", MODE="0666"
EOF

  if install -m 0644 /tmp/99-displaylink.rules "$UDEV_RULES_DEST"; then
    rm -f /tmp/99-displaylink.rules
    success "Udev rules installed"
  else
    rm -f /tmp/99-displaylink.rules
    error "Failed to install udev rules"
  fi
}

# Create systemd service
create_systemd_service() {
  if [[ "$INSTALL_SYSTEMD" != "true" ]]; then
    warn "Skipping systemd service installation"
    return 0
  fi

  log "Creating systemd service..."

  # Create temporary service file
  cat > /tmp/displaylink-driver.service <<EOF
[Unit]
Description=DisplayLink USB Driver
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$BINARY_DEST
Restart=on-failure
RestartSec=5
StandardOutput=append:/var/log/displaylink-driver.log
StandardError=append:/var/log/displaylink-driver.log
Environment="LD_LIBRARY_PATH=/usr/local/lib"
Environment="DISPLAYLINK_DRIVER_VERBOSE=0"

[Install]
WantedBy=multi-user.target
EOF

  if install -m 0644 /tmp/displaylink-driver.service "$SYSTEMD_SERVICE_DEST"; then
    rm -f /tmp/displaylink-driver.service
    success "Systemd service installed"

    # Reload systemd daemon
    systemctl daemon-reload
    success "Systemd daemon reloaded"
  else
    rm -f /tmp/displaylink-driver.service
    error "Failed to install systemd service"
  fi
}

# Create log directory
create_log_directory() {
  log "Creating log directory..."

  if mkdir -p /var/log && touch /var/log/displaylink-driver.log && chmod 666 /var/log/displaylink-driver.log; then
    success "Log directory created"
  else
    error "Failed to create log directory"
  fi
}

# Verify installation
verify_installation() {
  log "Verifying installation..."

  local errors=0

  if [[ ! -x "$BINARY_DEST" ]]; then
    error "Binary not executable at $BINARY_DEST"
    errors=$((errors + 1))
  fi

  if [[ "$INSTALL_UDEV" == "true" ]] && [[ ! -f "$UDEV_RULES_DEST" ]]; then
    warn "Udev rules not found at $UDEV_RULES_DEST"
    errors=$((errors + 1))
  fi

  if [[ "$INSTALL_SYSTEMD" == "true" ]] && [[ ! -f "$SYSTEMD_SERVICE_DEST" ]]; then
    warn "Systemd service not found at $SYSTEMD_SERVICE_DEST"
    errors=$((errors + 1))
  fi

  if [[ $errors -eq 0 ]]; then
    success "Installation verified"
  else
    error "Installation verification failed with $errors error(s)"
  fi
}

# Main installation process
main() {
  # Initialize log
  {
    echo "========================================"
    echo "DisplayLink Driver Installation Log"
    echo "Started: $(date)"
    echo "========================================"
  } > "$INSTALL_LOG"

  log "Starting installation"
  log "Script directory: $ROOT_DIR"

  # Check environment
  check_root
  verify_binary

  # Install components
  install_binary
  create_log_directory

  if [[ "$INSTALL_UDEV" == "true" ]]; then
    create_udev_rules
  fi

  if [[ "$INSTALL_SYSTEMD" == "true" ]]; then
    create_systemd_service
  fi

  # Verify
  verify_installation

  # Success message
  echo
  success "Installation completed successfully!"
  echo
  echo "Installed files:"
  echo "  Binary:          $BINARY_DEST"
  if [[ "$INSTALL_UDEV" == "true" ]]; then
    echo "  Udev rules:      $UDEV_RULES_DEST"
  fi
  if [[ "$INSTALL_SYSTEMD" == "true" ]]; then
    echo "  Systemd service: $SYSTEMD_SERVICE_DEST"
  fi
  echo "  Log file:        /var/log/displaylink-driver.log"
  echo
  echo "Next steps:"
  echo "  1. Reload udev rules:"
  echo "     sudo udevadm control --reload-rules && sudo udevadm trigger"
  echo
  echo "  2. Start the driver:"
  echo "     sudo systemctl start displaylink-driver"
  echo
  echo "  3. Check status:"
  echo "     sudo systemctl status displaylink-driver"
  echo "     tail -f /var/log/displaylink-driver.log"
  echo
}

# Run main function
main "$@"
