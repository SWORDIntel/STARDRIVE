#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}"

SUDO_CMD="sudo"
if [[ "$(id -u)" -eq 0 ]]; then
  SUDO_CMD=""
fi

run_as_root() {
  if [[ -n "$SUDO_CMD" ]]; then
    "$SUDO_CMD" "$@"
  else
    "$@"
  fi
}

find_displaylink_usb_path() {
  for dev in /sys/bus/usb/devices/*; do
    if [[ -f "$dev/idVendor" && -f "$dev/idProduct" ]]; then
      if [[ $(cat "$dev/idVendor") == "17e9" && $(cat "$dev/idProduct") == "4307" ]]; then
        basename "$dev"
        return 0
      fi
    fi
  done
  return 1
}

reset_displaylink_usb() {
  local path
  if path=$(find_displaylink_usb_path); then
    local auth="/sys/bus/usb/devices/$path/authorized"
    if [[ -w "$auth" ]]; then
      echo "Resetting USB device $path"
      run_as_root bash -c "echo 0 > '$auth' && sleep 0.1 && echo 1 > '$auth'"
    else
      echo "Cannot reset device: '$auth' not writable" >&2
    fi
  else
    echo "Warning: DisplayLink USB device not found for reset." >&2
  fi
}

restart_driver_instance() {
  local bin="/usr/local/bin/displaylink-driver"
  local log="/var/log/displaylink-driver.log"

  echo "  Attempting to restart driver instance..."

  # First, kill any running driver processes
  run_as_root pkill -f "$bin" >/dev/null 2>&1 || true
  echo "  Existing driver processes killed (if any)."

  # Unload the evdi kernel module
  echo "  Unloading evdi kernel module..."
  run_as_root modprobe -r evdi || echo "  Could not unload evdi module, it might not be loaded."

  # Reset the USB device
  reset_displaylink_usb

  # Load the new evdi kernel module
  echo "  Loading new evdi kernel module..."
  run_as_root modprobe evdi || echo "  Failed to load evdi module."

  # Restart the driver using nohup, similar to how build_driver.sh does it
  # Check if systemd is active first, if so, use systemctl
  if command -v systemctl &> /dev/null && systemctl is-system-running &> /dev/null; then
    echo "  Systemd detected, restarting service."
    run_as_root systemctl restart displaylink-driver
  else
    echo "  Systemd not detected or not running; launching driver directly via nohup."
    run_as_root mkdir -p /var/log
    run_as_root bash -c "LD_LIBRARY_PATH=/usr/local/lib DISPLAYLINK_DRIVER_VERBOSE=1 nohup '$bin' >'$log' 2>&1 &"
    run_as_root chmod 644 "$log"
    echo "  Driver started via nohup. Tail '$log' for output."
  fi
  sleep 2 # Give it a moment to start
  echo "  Driver restart initiated."
}

# Main execution for this script
if [[ "$EUID" -ne 0 ]]; then
  echo "This script needs to be run with sudo."
  exit 1
fi

case "$1" in
  restart)
    restart_driver_instance
    echo "Check /var/log/displaylink-driver.log for status."
    ;;
  *)
    echo "Usage: sudo ./manage_displaylink_driver.sh {restart}"
    exit 1
    ;;
esac
