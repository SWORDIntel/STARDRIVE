#!/usr/bin/env bash
#
# DisplayLink Driver Build Script
# Builds EVDI library, kernel module, and Rust driver
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}"
BUILD_LOG="${ROOT_DIR}/build.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SKIP_LIBRARY=${SKIP_LIBRARY:-false}
SKIP_MODULE=${SKIP_MODULE:-false}
SKIP_DRIVER=${SKIP_DRIVER:-false}
VERBOSE=${VERBOSE:-false}
RELEASE_MODE=${RELEASE_MODE:-true}

# Helper functions
log() {
  echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $*" | tee -a "$BUILD_LOG"
}

error() {
  echo -e "${RED}[ERROR]${NC} $*" | tee -a "$BUILD_LOG" >&2
  exit 1
}

success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*" | tee -a "$BUILD_LOG"
}

warn() {
  echo -e "${YELLOW}[WARN]${NC} $*" | tee -a "$BUILD_LOG"
}

usage() {
  cat <<'EOF'
Usage: ./build.sh [OPTIONS]

Build the DisplayLink driver and dependencies.

OPTIONS:
  --skip-library    Skip building EVDI library
  --skip-module     Skip building EVDI kernel module
  --skip-driver     Skip building Rust driver
  --debug           Build in debug mode (default: release)
  --verbose         Show all build output
  --help            Show this help message

EXAMPLES:
  ./build.sh                          # Full build
  ./build.sh --skip-module            # Skip kernel module
  ./build.sh --debug --verbose        # Debug with output

ENVIRONMENT VARIABLES:
  SKIP_LIBRARY      Skip EVDI library (export SKIP_LIBRARY=true)
  SKIP_MODULE       Skip EVDI module (export SKIP_MODULE=true)
  SKIP_DRIVER       Skip Rust driver (export SKIP_DRIVER=true)
  VERBOSE           Show output (export VERBOSE=true)
  RELEASE_MODE      Build release (export RELEASE_MODE=false for debug)

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-library) SKIP_LIBRARY=true ;;
    --skip-module) SKIP_MODULE=true ;;
    --skip-driver) SKIP_DRIVER=true ;;
    --debug) RELEASE_MODE=false ;;
    --verbose) VERBOSE=true ;;
    --help) usage; exit 0 ;;
    *)
      error "Unknown option: $1"
      ;;
  esac
  shift
done

# Check dependencies
check_dependencies() {
  local deps=("cargo" "gcc" "make" "pkg-config")
  local missing=()

  for cmd in "${deps[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
      missing+=("$cmd")
    fi
  done

  if [[ ${#missing[@]} -gt 0 ]]; then
    error "Missing dependencies: ${missing[*]}"
  fi

  log "All dependencies satisfied"
}

# Detect if we need sudo
check_sudo() {
  if [[ $EUID -ne 0 ]]; then
    if ! command -v sudo &> /dev/null; then
      error "sudo is required but not installed"
    fi
    SUDO_CMD="sudo"
  else
    SUDO_CMD=""
  fi
}

# Build EVDI library
build_evdi_library() {
  if [[ "$SKIP_LIBRARY" == "true" ]]; then
    warn "Skipping EVDI library build"
    return 0
  fi

  log "Building EVDI library..."

  if [[ ! -d "$ROOT_DIR/evdi_source/library" ]]; then
    error "EVDI library source not found at $ROOT_DIR/evdi_source/library"
  fi

  cd "$ROOT_DIR/evdi_source/library" || error "Cannot cd to evdi_source/library"

  if [[ "$VERBOSE" == "true" ]]; then
    make clean
    make
  else
    make clean > /dev/null 2>&1 || true
    make > /dev/null 2>&1 || error "EVDI library build failed"
  fi

  success "EVDI library built"
  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Install EVDI library
install_evdi_library() {
  if [[ "$SKIP_LIBRARY" == "true" ]]; then
    return 0
  fi

  log "Installing EVDI library..."

  cd "$ROOT_DIR/evdi_source/library" || error "Cannot cd to evdi_source/library"

  if $SUDO_CMD make install > /dev/null 2>&1; then
    success "EVDI library installed"
  else
    error "EVDI library installation failed"
  fi

  # Update library cache
  $SUDO_CMD ldconfig 2>/dev/null || true
  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Build EVDI kernel module
build_evdi_module() {
  if [[ "$SKIP_MODULE" == "true" ]]; then
    warn "Skipping EVDI kernel module build"
    return 0
  fi

  log "Building EVDI kernel module..."

  if [[ ! -d "$ROOT_DIR/evdi_source/module" ]]; then
    error "EVDI module source not found at $ROOT_DIR/evdi_source/module"
  fi

  cd "$ROOT_DIR/evdi_source/module" || error "Cannot cd to evdi_source/module"

  if [[ "$VERBOSE" == "true" ]]; then
    make clean
    make
  else
    make clean > /dev/null 2>&1 || true
    make > /dev/null 2>&1 || error "EVDI kernel module build failed"
  fi

  success "EVDI kernel module built"
  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Install EVDI kernel module
install_evdi_module() {
  if [[ "$SKIP_MODULE" == "true" ]]; then
    return 0
  fi

  log "Installing EVDI kernel module..."

  cd "$ROOT_DIR/evdi_source/module" || error "Cannot cd to evdi_source/module"

  if $SUDO_CMD make install > /dev/null 2>&1; then
    success "EVDI kernel module installed"
  else
    error "EVDI kernel module installation failed"
  fi

  # Load the module
  log "Loading EVDI kernel module..."
  if $SUDO_CMD modprobe evdi 2>/dev/null; then
    success "EVDI kernel module loaded"
  else
    warn "Could not load EVDI module (may already be loaded)"
  fi

  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Build Rust driver
build_rust_driver() {
  if [[ "$SKIP_DRIVER" == "true" ]]; then
    warn "Skipping Rust driver build"
    return 0
  fi

  log "Building Rust driver..."

  if [[ ! -d "$ROOT_DIR/displaylink-driver" ]]; then
    error "Rust driver source not found at $ROOT_DIR/displaylink-driver"
  fi

  cd "$ROOT_DIR/displaylink-driver" || error "Cannot cd to displaylink-driver"

  local build_cmd="cargo build"
  if [[ "$RELEASE_MODE" == "true" ]]; then
    build_cmd="$build_cmd --release"
  fi

  if [[ "$VERBOSE" == "true" ]]; then
    $build_cmd || error "Rust driver build failed"
  else
    $build_cmd > /dev/null 2>&1 || error "Rust driver build failed"
  fi

  # Verify binary was created
  if [[ "$RELEASE_MODE" == "true" ]]; then
    BINARY="$ROOT_DIR/displaylink-driver/target/release/displaylink-driver"
  else
    BINARY="$ROOT_DIR/displaylink-driver/target/debug/displaylink-driver"
  fi

  if [[ ! -f "$BINARY" ]]; then
    error "Binary not found at $BINARY"
  fi

  success "Rust driver built: $BINARY"
  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Run tests
run_tests() {
  log "Running tests..."

  cd "$ROOT_DIR/displaylink-driver" || error "Cannot cd to displaylink-driver"

  if [[ "$VERBOSE" == "true" ]]; then
    cargo test --release || error "Tests failed"
  else
    cargo test --release > /dev/null 2>&1 || error "Tests failed"
  fi

  success "All tests passed"
  cd "$ROOT_DIR" || error "Cannot return to root directory"
}

# Main build process
main() {
  log "Starting DisplayLink driver build"
  log "Build directory: $ROOT_DIR"
  log "Log file: $BUILD_LOG"

  # Initialize log
  {
    echo "========================================"
    echo "DisplayLink Driver Build Log"
    echo "Started: $(date)"
    echo "========================================"
  } > "$BUILD_LOG"

  # Check environment
  check_dependencies
  check_sudo

  # Build sequence
  build_evdi_library
  install_evdi_library

  build_evdi_module
  install_evdi_module

  build_rust_driver
  run_tests

  # Final summary
  echo
  success "Build completed successfully!"
  echo
  echo "Summary:"
  echo "  EVDI Library:  $([ "$SKIP_LIBRARY" = "true" ] && echo "Skipped" || echo "Built & Installed")"
  echo "  EVDI Module:   $([ "$SKIP_MODULE" = "true" ] && echo "Skipped" || echo "Built & Installed")"
  echo "  Rust Driver:   $([ "$SKIP_DRIVER" = "true" ] && echo "Skipped" || echo "Built")"
  echo
  if [[ "$SKIP_DRIVER" != "true" ]]; then
    echo "Next steps:"
    echo "  1. Review configuration in INSTALL.md"
    echo "  2. Run: ./install.sh (as root or with sudo)"
    echo "  3. Test: sudo ./target/release/displaylink-driver"
    echo
  fi
}

# Run main function
main "$@"
