#!/usr/bin/env bash
set -euo pipefail

ROOT="/home/john/Documents/STARDRIVE"
ARTIFACTS="${ROOT}/logs"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
SESSION_DIR="${ARTIFACTS}/run-${TIMESTAMP}"

mkdir -p "$SESSION_DIR"

BUILD_LOG="${SESSION_DIR}/build.log"
DRIVER_LOG="${SESSION_DIR}/displaylink-driver.log"
LSUSB_LOG="${SESSION_DIR}/lsusb.log"
USBDEVICES_LOG="${SESSION_DIR}/usb-devices.log"
DMESG_LOG="${SESSION_DIR}/dmesg.log"
JOURNAL_LOG="${SESSION_DIR}/journal.log"
SYSLOG_LOG="${SESSION_DIR}/syslog.log"
XRANDR_LOG="${SESSION_DIR}/xrandr.log"
USBMON_LOG="${SESSION_DIR}/usbmon.log"
LSMOD_LOG="${SESSION_DIR}/lsmod.log"

echo "Artifacts will be stored in $SESSION_DIR"

pushd "$ROOT" > /dev/null
{
  ./build_driver.sh
} 2>&1 | tee "$BUILD_LOG"
popd > /dev/null

if [[ -f /var/log/displaylink-driver.log ]]; then
  tail -n 200 /var/log/displaylink-driver.log > "$DRIVER_LOG"
fi

lsusb > "$LSUSB_LOG"
usb-devices > "$USBDEVICES_LOG"
dmesg | tail -n 80 > "$DMESG_LOG"
journalctl -n 80 -u displaylink-driver > "$JOURNAL_LOG" || true
sudo tail -n 80 /var/log/syslog > "$SYSLOG_LOG" || true
xrandr --verbose > "$XRANDR_LOG" 2>/dev/null || true
lsmod > "$LSMOD_LOG"

if [[ -d /sys/kernel/debug/usb/usbmon ]]; then
  sudo timeout 5 cat /sys/kernel/debug/usb/usbmon/0u > "$USBMON_LOG" 2>/dev/null || true
fi

echo "Captured run artifacts:" >&2
ls -1 "$SESSION_DIR"
