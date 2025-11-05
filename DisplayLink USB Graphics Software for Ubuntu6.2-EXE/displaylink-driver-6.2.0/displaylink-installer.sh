#!/usr/bin/env bash
# Copyright (c) 2015 - 2024 DisplayLink (UK) Ltd.

export LC_ALL=C
readonly SELF=$0
readonly COREDIR=/opt/displaylink
readonly LOGSDIR=/var/log/displaylink
readonly PRODUCT="DisplayLink Linux Software"
VERSION=6.2.0
ACTION=install
NOREBOOT=false

DEB_DEPENDENCIES=(mokutil pkg-config libdrm-dev libc6-dev coreutils)
if grep -Ei 'raspb(erry|ian)' /proc/cpuinfo /etc/os-release &>/dev/null ; then
  DEB_DEPENDENCIES+=(raspberrypi-kernel-headers)
fi
readonly DEB_DEPENDENCIES

prompt_yes_no()
{
  read -rp "$1 (Y/n) " CHOICE
  [[ ! ${CHOICE:-Y} == "${CHOICE#[Yy]}" ]]
}

prompt_command()
{
  echo "> $*"
  prompt_yes_no "Do you want to continue?" || exit 0
  "$@"
}

error()
{
  echo "ERROR: $*" >&2
}


install_evdi()
{
  local TARGZ=$1
  local EVDI=$2

  if ! tar xf "$TARGZ" -C "$EVDI"; then
    error "Unable to extract $TARGZ to $EVDI"
    return 1
  fi

  echo "[[ Installing EVDI DKMS module ]]"

  # shellcheck source=/dev/null
  source "$EVDI"/module/dkms_install.sh
  evdi_dkms_install || return
  evdi_add_mod_options

  echo "[[ Installing EVDI library ]]"

  if ! make -C "${EVDI}/library"; then
    error "Failed to build evdi library."
    return 1
  fi

  if ! install "${EVDI}/library/libevdi.so" "$COREDIR"; then
    error "Failed to copy evdi library to $COREDIR."
    return 1
  fi
}

uninstall_evdi_module()
{
  local TARGZ=$1
  local EVDI=$2

  if ! tar xf "$TARGZ" -C "$EVDI"; then
    error "Unable to extract $TARGZ to $EVDI"
    return 1
  fi

  make -C "${EVDI}/module" uninstall_dkms
}

is_32_bit()
{
  [[ $(getconf LONG_BIT) == 32 ]]
}

is_armv7()
{
  grep -qi -F armv7 /proc/cpuinfo
}

is_armv8()
{
  [[ "$(uname -m)" == "aarch64" ]]
}

cleanup_logs()
{
  rm -rf "$LOGSDIR"
}

cleanup()
{
  rm -rf "$COREDIR"
  rm -f /usr/bin/displaylink-installer
  rm -f /usr/bin/DLSupportTool.sh
  rm -f ~/.dl.xml
  rm -f /root/.dl.xml
  rm -f /etc/modprobe.d/evdi.conf
  rm -rf /etc/modules-load.d/evdi.conf
}

binary_location()
{
  if is_armv7; then
    echo "arm-linux-gnueabihf"
  elif is_armv8; then
     echo "aarch64-linux-gnu"
  else
    local PREFIX="x64"
    local POSTFIX="ubuntu-1604"

    is_32_bit && PREFIX="x86"
    echo "$PREFIX-$POSTFIX"
  fi
}

install_with_standalone_installer()
{
  local scriptDir
  scriptDir=$(dirname "$(realpath "${BASH_SOURCE[0]}")")
  if [[ $scriptDir == "$COREDIR" ]]; then
    error "DisplayLink driver is already installed"
    exit 1
  fi

  printf '\n%s\n' "Installing"

  install -d "$COREDIR" "$LOGSDIR"

  install "$SELF" "$COREDIR"
  ln -sf "$COREDIR/$(basename "$SELF")" /usr/bin/displaylink-installer

  install DLSupportTool.sh /usr/bin/DLSupportTool.sh

  echo "[ Installing EVDI ]"

  local temp_dir
  temp_dir=$(mktemp -d)
  finish() {
    rm -rf "$temp_dir"
  }
  trap finish EXIT

  local evdi_dir="$temp_dir/evdi"
  mkdir "$evdi_dir"

  if ! install_evdi 'evdi.tar.gz' "$evdi_dir"; then
    cleanup
    finish
    exit 1
  fi

  finish

  local BINS DLM LIBUSB_SO LIBUSB_PATH
  BINS=$(binary_location)
  DLM="$BINS/DisplayLinkManager"
  LIBUSB_SO="libusb-1.0.so.0.3.0"
  LIBUSB_PATH="$BINS/$LIBUSB_SO"

  install -m 0644 'evdi.tar.gz' "$COREDIR"

  echo "[ Installing $DLM ]"
  install "$DLM" "$COREDIR"

  echo "[ Installing libraries ]"
  install "$LIBUSB_PATH" "$COREDIR"
  ln -sf "$LIBUSB_SO" "$COREDIR/libusb-1.0.so.0"
  ln -sf "$LIBUSB_SO" "$COREDIR/libusb-1.0.so"

  echo "[ Installing firmware packages ]"
  install -m 0644 ./*.spkg "$COREDIR"

  echo "[ Installing licence file ]"
  install -m 0644 LICENSE "$COREDIR"
  if [[ -f 3rd_party_licences.txt ]]; then
    install -m 0644 3rd_party_licences.txt "$COREDIR"
  fi

  source udev-installer.sh
  local displaylink_bootstrap_script="$COREDIR/udev.sh"
  create_bootstrap_file "$SYSTEMINITDAEMON" "$displaylink_bootstrap_script"

  echo "[ Adding udev rule for DisplayLink DL-3xxx/4xxx/5xxx/6xxx devices ]"
  create_udev_rules_file "/usr/lib/udev/rules.d/99-displaylink.rules"
  xorg_running || udevadm control -R
  xorg_running || udevadm trigger

  echo "[ Adding upstart and powermanager sctripts ]"
  source service-installer.sh
  create_dl_service "$SYSTEMINITDAEMON" "$COREDIR"

  install -m 0644 service-installer.sh "$COREDIR"

  xorg_running || trigger_udev_if_devices_connected

  xorg_running || "$displaylink_bootstrap_script" START

  printf '\n%s\n%s\n' "Please read the FAQ" \
        "http://support.displaylink.com/knowledgebase/topics/103927-troubleshooting-ubuntu"

  "$NOREBOOT" && exit 0

  evdi_success_message
  printf '%s\n\n' "DisplayLink driver installed successfully."

  if evdi_requires_reboot && prompt_yes_no "Do you want to reboot now?"; then
     reboot
  fi

  exit 0
}

apt_curl_install()
{
  if ! program_exists curl; then
    apt_ask_for_update
    apt_ask_for_upgrade
    apt install curl
  fi
}

install_deb()
{
  dpkg -i ./synaptics-repository-keyring.deb
  apt_ask_for_update
  apt install displaylink-driver
}

install_synaptics_repository_keyring()
{
  curl https://www.synaptics.com/sites/default/files/Ubuntu/pool/stable/main/all/synaptics-repository-keyring.deb --output synaptics-repository-keyring.deb
}

uninstall_synaptics_repository_keyring()
{
  rm synaptics-repository-keyring.deb
}

install_with_apt()
{
  apt_curl_install
  install_synaptics_repository_keyring

  install_deb

  uninstall_synaptics_repository_keyring
}

uninstall_apt()
{
  program_exists apt || return 0

  if check_installed displaylink-driver; then
    echo "'displaylink-driver' debian detected: removing..."
    apt remove displaylink-driver
  fi

  if check_installed evdi; then
    echo "'evdi' debian detected: removing..."
    apt remove evdi
  fi
}

uninstall_standalone()
{
  [[ -e /opt/displaylink/evdi.tar.gz ]] || return 0
  printf '\n%s\n\n' "Uninstalling"

  echo "[ Removing EVDI from kernel tree, DKMS, and removing sources. ]"

  cd "$(dirname "$(realpath "${BASH_SOURCE[0]}")")" || exit

  local temp_dir
  temp_dir=$(mktemp -d)
  uninstall_evdi_module "evdi.tar.gz" "$temp_dir"
  rm -rf "$temp_dir"

  source service-installer.sh

  case $SYSTEMINITDAEMON in
    upstart)
      remove_upstart_service ;;
    systemd)
      remove_systemd_service ;;
    runit)
      remove_runit_service ;;
  esac

  echo "[ Removing suspend-resume hooks ]"
  remove_pm_scripts

  echo "[ Removing udev rule ]"
  rm -f /etc/udev/rules.d/99-displaylink.rules
  rm -f /usr/lib/udev/rules.d/99-displaylink.rules
  udevadm control -R
  udevadm trigger

  echo "[ Removing Core folder ]"
  cleanup
  cleanup_logs

  printf '\n%s\n' "Uninstallation steps complete."
  if [[ -f /sys/devices/evdi/version ]]; then
    echo "Please note that the evdi kernel module is still in the memory."
    echo "A reboot is required to fully complete the uninstallation process."
  fi
}

uninstall()
{
  uninstall_apt
  check_requirements
  uninstall_standalone
}

missing_requirement()
{
  echo "Unsatisfied dependencies. Missing component: $1." >&2
  echo "This is a fatal error, cannot install $PRODUCT." >&2
  exit 1
}

version_lt()
{
  local left
  left=$(echo "$1" | cut -d. -f-2)
  local right
  right=$(echo "$2" | cut -d. -f-2)

  local greater
  greater=$(printf '%s\n%s' "$left" "$right" | sort -Vr | head -1)

  [[ "$greater" != "$left" ]]
}

program_exists()
{
  command -v "${1:?}" >/dev/null
}

install_dependencies()
{
  program_exists apt || return 0
  install_dependencies_apt
}

check_installed()
{
  program_exists apt || return 0
  apt list -qq --installed "${1:?}" 2>/dev/null | sed 's:/.*$::' | grep -q -F "$1"
}

apt_ask_for_dependencies()
{
  apt --simulate install "$@" 2>&1 |  grep  "^E: " > /dev/null && return 1
  apt --simulate install "$@" | grep -v '^Inst\|^Conf'
}

apt_ask_for_update()
{
  echo "Need to update package list."
  prompt_yes_no "apt update?" || return 1
  apt update
}

apt_ask_for_upgrade()
{
  echo "Need to upgrade package list."
  prompt_yes_no "apt upgrade?" || return 1
  apt upgrade
}

install_dependencies_apt()
{
  local packages=()
  program_exists dkms || packages+=(dkms)

  for item in "${DEB_DEPENDENCIES[@]}"; do
    check_installed "$item" || packages+=("$item")
  done

  if [[ ${#packages[@]} -gt 0 ]]; then
    echo "[ Installing dependencies ]"

    if ! apt_ask_for_dependencies "${packages[@]}"; then
      # shellcheck disable=SC2015
      apt_ask_for_update && apt_ask_for_dependencies "${packages[@]}" || check_requirements
    fi

    prompt_command apt install -y "${packages[@]}" || check_requirements
  fi
}

uninstall_older_version()
{
  local local_version
  local dl_bin=/opt/displaylink/DisplayLinkManager
  
  [[ -f "$dl_bin" ]] || return
  
  local_version=$("$dl_bin" -version | cut -d' ' -f2 | cut -b2-)
  echo "Uninstalling older displaylink-driver v${local_version}"
  uninstall
}

perform_install_steps()
{
  install_dependencies
  check_requirements
  uninstall_older_version

  if program_exists apt && prompt_yes_no "Do you want to install with apt?"; then
    install_with_apt
  else
    install_with_standalone_installer
  fi
}

install_and_save_log()
{
  local install_log_path="${LOGSDIR}/displaylink_installer.log"

  install -d "$LOGSDIR"

  perform_install_steps 2>&1 | tee -a "$install_log_path"
}

check_requirements()
{
  local missing=()
  program_exists dkms || missing+=("DKMS")

  for item in "${DEB_DEPENDENCIES[@]}"; do
    check_installed "$item" || missing+=("${item%-dev}")
  done

  [[ ${#missing[@]} -eq 0 ]] || missing_requirement "${missing[*]}"

  # Required kernel version
  local KVER
  KVER=$(uname -r)
  local KVER_MIN="4.15"
  version_lt "$KVER" "$KVER_MIN" && missing_requirement "Kernel version $KVER is too old. At least $KVER_MIN is required"

  # Linux headers
  [[ -d "/lib/modules/$KVER/build" ]] || missing_requirement "Linux headers for running kernel, $KVER"
}

usage()
{
  echo
  echo "Installs $PRODUCT, version $VERSION."
  echo "Usage: $SELF [ install | uninstall | noreboot | version ]"
  echo
  echo "The default operation is install."
  echo "If unknown argument is given, a quick compatibility check is performed but nothing is installed."
  exit 1
}

detect_init_daemon()
{
  local init
  init=$(readlink /proc/1/exe)

  if [[ $init == "/sbin/init" ]]; then
    init=$(/sbin/init --version)
  fi

  case $init in
    *upstart*)
      SYSTEMINITDAEMON="upstart" ;;
    *systemd*)
      SYSTEMINITDAEMON="systemd" ;;
    *runit*)
      SYSTEMINITDAEMON="runit" ;;
    *)
      echo "ERROR: the installer script is unable to find out how to start DisplayLinkManager service automatically on your system." >&2
      echo "Please set an environment variable SYSTEMINITDAEMON to 'upstart', 'systemd' or 'runit' before running the installation script to force one of the options." >&2
      echo "Installation terminated." >&2
      exit 1
  esac
}

unsupported_distro_message()
{
    echo "WARNING: This is not an officially supported distribution." >&2
    echo "Please use DisplayLink Forum for getting help if you find issues." >&2
}

check_supported_libc_version()
{
  local MIN_GLIBC_MAJOR=2
  local MIN_GLIBC_MINOR=31
  local regex='.*([[:digit:]]+)\.([[:digit:]]+)'
  local ldd_version

  ldd_version=$(ldd --version | head -n 1)

  if [[ $ldd_version =~ $regex ]]; then
    major=${BASH_REMATCH[1]}
    minor=${BASH_REMATCH[2]}

    [[ $major -eq $MIN_GLIBC_MAJOR ]] && [[ $minor -ge $MIN_GLIBC_MINOR ]] && return 0
    [[ $major -gt $MIN_GLIBC_MAJOR ]] && return 0
  fi

  echo "Unsupported libc version. Minimal supported: 2.31" >&2
  return 1
}

check_supported_ubuntu_version_or_exit()
{
  local MIN_UBUNTU_MAJOR=20
  local regex='^Ubuntu[[:space:]]+([[:digit:]]+)'

  if [[ $(lsb_release -d -s) =~ $regex ]] ; then
    local os_version_major=${BASH_REMATCH[1]}

    if [[ $os_version_major -lt $MIN_UBUNTU_MAJOR ]]; then
      echo "Unsupported Ubuntu version" >&2

      if check_supported_libc_version 2>/dev/null; then
        unsupported_distro_message
        return 0
      fi

      exit 1
    fi
  else
    unsupported_distro_message
  fi
}

check_distro_compliance_or_exit()
{
  if hash lsb_release 2>/dev/null; then
    echo -n "Distribution discovered: "
    lsb_release -d -s

    check_supported_ubuntu_version_or_exit
    check_supported_libc_version || exit 1

  else
    unsupported_distro_message
  fi
}

xorg_running()
{
  local SESSION_NO
  SESSION_NO=$(loginctl | awk "/$(logname)/ {print \$1; exit}")
  [[ $(loginctl show-session "$SESSION_NO" -p Type) == *=x11 ]]
}

if [[ $(id -u) != "0" ]]; then
  echo "You need to be root to use this script." >&2
  exit 1
fi

[[ -z $SYSTEMINITDAEMON ]] && detect_init_daemon || echo "Trying to use the forced init system: $SYSTEMINITDAEMON"
check_distro_compliance_or_exit

while [[ $# -gt 0 ]]; do
  case "$1" in
    install)
      ACTION="install"
      ;;

    uninstall)
      ACTION="uninstall"
      ;;
    noreboot)
      NOREBOOT=true
      ;;
    version)
      ACTION="version"
      ;;
    *)
      usage
      ;;
  esac
  shift
done

case "$ACTION" in
  install)
    install_and_save_log
    ;;

  uninstall)
    uninstall
    ;;

  version)
    echo "$PRODUCT $VERSION"
    ;;
esac
