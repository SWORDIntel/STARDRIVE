#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}"

SKIP_LIBRARY=false
SKIP_MODULE=false
SKIP_DRIVER=false
SKIP_INSTALL=false
SKIP_ACTIVATE=false
AUTO_RESTARTS=3
ERROR_PATTERNS=("Channel init failed" "Failed to initialize device" "Pipe error")

usage() {
  cat <<'EOF'
Usage: ./build_driver.sh [options]

Options:
  --skip-library    Skip building the EVDI library components.
  --skip-module     Skip building/installing the EVDI kernel module.
  --skip-driver     Skip compiling the Rust DisplayLink driver.
  --skip-install    Skip installing the compiled binary into /usr/local/bin.
  --skip-activate   Skip creating/enabling the systemd service or running the binary.
  --help            Show this help text.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-library) SKIP_LIBRARY=true ;;
    --skip-module) SKIP_MODULE=true ;;
    --skip-driver) SKIP_DRIVER=true ;;
    --skip-install) SKIP_INSTALL=true ;;
    --skip-activate) SKIP_ACTIVATE=true ;;
    --help) usage; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
  shift
done

SUDO_CMD="sudo"
if [[ "$(id -u)" -eq 0 ]]; then
  SUDO_CMD=""
fi

DEPENDENCIES=(make cargo gcc pkg-config openssl)

require_command() {
  if ! command -v "$1" &> /dev/null; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

check_dependencies() {
  local missing=()
  for cmd in "${DEPENDENCIES[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
      missing+=("$cmd")
    fi
  done

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Missing dependencies: ${missing[*]}" >&2
    echo "Install them via your package manager before continuing." >&2
    exit 1
  fi
}

if [[ -n "$SUDO_CMD" ]]; then
  require_command sudo
fi

check_dependencies

run_as_root() {
  if [[ -n "$SUDO_CMD" ]]; then
    "$SUDO_CMD" "$@"
  else
    "$@"
  fi
}

find_common_headers_dir() {
  local release="${1:-$(uname -r)}"
  local base="${release%-*}"
  local candidate="/usr/src/linux-headers-${base}-common"

  if [[ -d "$candidate" ]]; then
    echo "$candidate"
    return 0
  fi

  local match=""
  shopt -s nullglob
  for dir in /usr/src/linux-headers-*common; do
    if [[ "$dir" == *"$base"* ]]; then
      match="$dir"
      break
    fi
  done
  shopt -u nullglob

  if [[ -n "$match" ]]; then
    echo "$match"
    return 0
  fi

  return 1
}

prepare_module_signing() {
  local kernel_release
  kernel_release=$(uname -r)
  local build_dir
  build_dir=$(readlink -f "/lib/modules/${kernel_release}/build" 2>/dev/null || true)

  if [[ -n "$build_dir" && -d "$build_dir" ]]; then
    local system_map_src="/boot/System.map-${kernel_release}"
    local system_map_target="${build_dir}/System.map"
    if [[ -f "$system_map_src" && ! -f "$system_map_target" ]]; then
      echo "Installing System.map for $kernel_release"
      run_as_root cp "$system_map_src" "$system_map_target"
    elif [[ ! -f "$system_map_src" ]]; then
      echo "Warning: ${system_map_src} missing; depmod may skip."
    fi
  else
    echo "Warning: Unable to resolve kernel build directory for ${kernel_release}"
  fi

  local headers_dir
  local signing_dir=""
  if [[ -d "${build_dir}/certs" ]]; then
    signing_dir="${build_dir}/certs"
  else
    signing_dir="${build_dir}/certs"
    run_as_root mkdir -p "$signing_dir"
  fi

  local key="${signing_dir}/signing_key.pem"
  local cert="${signing_dir}/signing_key.x509"

  if [[ ! -f "$key" ]] || [[ ! -f "$cert" ]]; then
    echo "Generating module signing key for ${kernel_release}"
    run_as_root openssl req -new -x509 -newkey rsa:2048 -sha256 -days 3650 -nodes \
      -subj "/CN=evdi-module-signing/" \
      -keyout "$key" -out "$cert"
    run_as_root chmod 600 "$key"
    run_as_root chmod 644 "$cert"
  else
    echo "Module signing artifacts already installed at ${signing_dir}"
  fi

  if headers_dir=$(find_common_headers_dir "$kernel_release"); then
    local common_output="${headers_dir}/output"
    if [[ ! -d "$common_output" ]]; then
      run_as_root mkdir -p "$common_output"
    fi
    run_as_root ln -sf "$key" "${common_output}/signing_key.pem" || true
    run_as_root ln -sf "$cert" "${common_output}/signing_key.x509" || true
  fi
}

apply_module_blacklist() {
  local blacklist_conf="/etc/modprobe.d/displaylink-blacklist.conf"

  if [[ ! -f "$blacklist_conf" ]] || ! grep -q '^blacklist udl' "$blacklist_conf"; then
    echo "Ensuring conflicting modules are blacklisted"
    run_as_root bash -c "cat <<EOF > '$blacklist_conf'
blacklist udl
EOF"
  fi

  run_as_root modprobe -r udl >/dev/null 2>&1 || true
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

unbind_displaylink_udl() {
  local path
  if path=$(find_displaylink_usb_path); then
    local unbind="/sys/bus/usb/drivers/udl/unbind"
    if [[ -w "$unbind" ]]; then
      echo "Forcing udl driver off $path"
      run_as_root bash -c "echo -n '$path' > '$unbind'"
    else
      echo "udl unbind interface not available; skipping"
    fi
  fi
}

reset_displaylink_usb() {
  local path
  if path=$(find_displaylink_usb_path); then
    local auth="/sys/bus/usb/devices/$path/authorized"
    if [[ -w "$auth" ]]; then
      echo "Resetting USB device $path"
      run_as_root bash -c "echo 0 > '$auth' && sleep 0.1 && echo 1 > '$auth'"
    fi
  fi
}

apply_udev_workaround() {
  local rule="/etc/udev/rules.d/99-displaylink-driver.rules"

  if [[ ! -f "$rule" ]] || ! grep -q "DisplayLink USB 3.0 Dual Video Dock" "$rule"; then
    echo "Writing udev rule to keep udl unbound"
    run_as_root bash -c "cat <<'EOF' > '$rule'
ACTION==\"add\", SUBSYSTEM==\"usb\", ATTR{idVendor}==\"17e9\", ATTR{idProduct}==\"4307\", RUN+=\"/bin/sh -c 'echo -n %k > /sys/bus/usb/drivers/udl/unbind 2>/dev/null || true'\"
EOF"
    run_as_root udevadm control --reload-rules
    run_as_root udevadm trigger
  fi
}

build_library() {
  echo "==> Building EVDI user-space library"
  pushd "$ROOT_DIR/evdi_source/library" > /dev/null
  make
  run_as_root make install
  run_as_root bash -c 'printf "/usr/local/lib\n" > /etc/ld.so.conf.d/displaylink.conf'
  run_as_root ldconfig
  popd > /dev/null
}

setup_module_autoload() {
  run_as_root mkdir -p /etc/modules-load.d
  run_as_root bash -c 'echo evdi > /etc/modules-load.d/evdi.conf'
}

build_module() {
  echo "==> Building/installing the EVDI kernel module"
  pushd "$ROOT_DIR/evdi_source/module" > /dev/null
  prepare_module_signing
  apply_module_blacklist
  apply_udev_workaround
  unbind_displaylink_udl
  reset_displaylink_usb
  make
  run_as_root make install
  run_as_root depmod -a
  run_as_root modprobe evdi || echo "modprobe evdi failed; check kernel version" >&2
  setup_module_autoload
  popd > /dev/null
}

build_driver() {
  echo "==> Building DisplayLink Rust driver"
  pushd "$ROOT_DIR/displaylink-driver" > /dev/null
  cargo fmt
  cargo build --release
  popd > /dev/null
}

run_health_summary() {
  echo "==> Performing driver health check"
  local log="/var/log/displaylink-driver.log"
  local service_active=false

  if command -v systemctl &> /dev/null && systemctl list-unit-files displaylink-driver.service >/dev/null 2>&1; then
    service_active=true
    if run_as_root systemctl is-active --quiet displaylink-driver; then
      echo "  displaylink-driver.service is active"
    else
      echo "  displaylink-driver.service failed to start" >&2
      run_as_root systemctl restart displaylink-driver || echo "  Restart failed" >&2
    fi

    if run_as_root journalctl --no-pager -n 20 -u displaylink-driver | grep -q "Device initialized successfully"; then
      echo "  Device initialization confirmed in journal"
    else
      echo "  No confirmation found in journal yet"
    fi
  fi

  if ! $service_active; then
    local tries=0
    until pgrep -f "/usr/local/bin/displaylink-driver" >/dev/null 2>&1 || [[ $tries -ge 10 ]]; do
      sleep 1
      ((tries++))
    done

    if [[ $tries -lt 10 ]]; then
      echo "  displaylink-driver process running (pid=$(pgrep -f "/usr/local/bin/displaylink-driver"))"
    else
      echo "  displaylink-driver process not detected" >&2
    fi

    if [[ -f $log ]]; then
      if grep -q "Device initialized successfully" "$log"; then
        echo "  Device initialization confirmed in $log"
      else
        echo "  Device initialization message not found in $log"
      fi
    else
      echo "  Log file $log missing" >&2
    fi
  fi
}

check_driver_health() {
  run_health_summary
  local log="/var/log/displaylink-driver.log"
  if [[ -f "$log" ]]; then
    echo "==== Last 12 log lines ===="
    tail -n 12 "$log"
  fi
}

restart_driver_instance() {
  local bin="/usr/local/bin/displaylink-driver"

  if command -v systemctl &> /dev/null && systemctl is-system-running &> /dev/null; then
    echo "  Restarting systemd service"
    reset_displaylink_usb
    run_as_root systemctl restart displaylink-driver
    sleep 2
    return
  fi

  echo "  Restarting nohup-launched driver"
  run_as_root pkill -f "$bin" >/dev/null 2>&1 || true
  reset_displaylink_usb
  run_as_root bash -c "LD_LIBRARY_PATH=/usr/local/lib DISPLAYLINK_DRIVER_VERBOSE=1 nohup '$bin' >/var/log/displaylink-driver.log 2>&1 &"
  sleep 2
}

monitor_and_retry_driver() {
  local log="/var/log/displaylink-driver.log"
  if [[ ! -f "$log" ]]; then
    return
  fi

  local regex
  regex=$(IFS='|'; echo "${ERROR_PATTERNS[*]}")

  local attempt=0
  while (( attempt < AUTO_RESTARTS )); do
    if tail -n 60 "$log" | grep -E "$regex" >/dev/null 2>&1; then
      ((attempt++))
      echo "  Detected error pattern in log (attempt $attempt/${AUTO_RESTARTS}); restarting driver"
      restart_driver_instance
      run_health_summary
      continue
    fi
    break
  done

  if (( attempt == AUTO_RESTARTS )); then
    echo "  Driver still failing after ${AUTO_RESTARTS} retries; manual inspection required"
  fi
}

install_driver_binary() {
  local src="$ROOT_DIR/displaylink-driver/target/release/displaylink-driver"
  local dest="/usr/local/bin/displaylink-driver"

  if [[ ! -x "$src" ]]; then
    echo "Driver binary not found at $src – run `cargo build --release` first" >&2
    exit 1
  fi

  echo "Installing driver binary to $dest"
  run_as_root install -Dm755 "$src" "$dest"
}

activate_driver_service() {
  local bin="/usr/local/bin/displaylink-driver"
  local service_path="/etc/systemd/system/displaylink-driver.service"

  if command -v systemctl &> /dev/null && systemctl is-system-running &> /dev/null; then
    echo "Configuring systemd service for DisplayLink driver"
    cat <<EOF | run_as_root tee "$service_path" >/dev/null
[Unit]
Description=DisplayLink Rust driver
After=systemd-modules-load.service
Wants=systemd-modules-load.service

[Service]
Type=simple
Environment=LD_LIBRARY_PATH=/usr/local/lib
Environment=DISPLAYLINK_DRIVER_VERBOSE=1
ExecStart=$bin
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
    unbind_displaylink_udl
    unbind_displaylink_udl
    reset_displaylink_usb
    run_as_root systemctl daemon-reload
    run_as_root systemctl enable --now displaylink-driver.service
    run_as_root chmod 644 /var/log/displaylink-driver.log
    run_health_summary
    return
  fi

  echo "systemd not detected or not running; launching driver directly"
  run_as_root mkdir -p /var/log
  reset_displaylink_usb
  run_as_root bash -c "LD_LIBRARY_PATH=/usr/local/lib DISPLAYLINK_DRIVER_VERBOSE=1 nohup '$bin' >/var/log/displaylink-driver.log 2>&1 &"
  run_as_root chmod 644 /var/log/displaylink-driver.log
  echo "Driver started via nohup; tail /var/log/displaylink-driver.log for output"
  check_driver_health
}

echo "Running build_driver.sh from $ROOT_DIR"

if [[ "$SKIP_LIBRARY" == "false" ]]; then
  build_library
else
  echo "Skipping EVDI library build (--skip-library)"
fi

if [[ "$SKIP_MODULE" == "false" ]]; then
  build_module
else
  echo "Skipping EVDI module build (--skip-module)"
fi

if [[ "$SKIP_DRIVER" == "false" ]]; then
  build_driver
else
  echo "Skipping Rust driver build (--skip-driver)"
fi

if [[ "$SKIP_INSTALL" == "false" ]]; then
  install_driver_binary
else
  echo "Skipping driver installation (--skip-install)"
fi

if [[ "$SKIP_ACTIVATE" == "false" ]]; then
  activate_driver_service
  monitor_and_retry_driver
else
  echo "Skipping driver activation (--skip-activate)"
fi

echo "Build finished. Inspect logs if something failed."
