#!/usr/bin/env bash
# Copyright (c) 2022 DisplayLink (UK) Ltd.

add_upstart_service()
{
  local conf_dir="${1:-/}"
  conf_dir="${conf_dir%/}/etc/init"
  mkdir -p "$conf_dir"

  local conf_path="$conf_dir/displaylink-driver.conf"
  cat > "$conf_path" <<EOF
description "DisplayLink Driver Service"
# Copyright (c) 2015 - 2019 DisplayLink (UK) Ltd.

start on login-session-start
stop on desktop-shutdown

# Restart if process crashes
respawn

# Only attempt to respawn 10 times in 5 seconds
respawn limit 10 5

chdir /opt/displaylink

pre-start script
    . /opt/displaylink/udev.sh

    if [ "\$(get_displaylink_dev_count)" = "0" ]; then
        stop
        exit 0
    fi
end script

script
    [ -r /etc/default/displaylink ] && . /etc/default/displaylink
    modprobe evdi || (dkms install \$(ls -t /usr/src | grep evdi | head -n1  | sed -e "s:-:/:") && modprobe evdi)
    exec /opt/displaylink/DisplayLinkManager
end script
EOF

  chmod 0644 "$conf_path"
}

add_systemd_service()
{
  local service_dir="${1:-/}"
  service_dir="${service_dir%/}/lib/systemd/system"
  mkdir -p "$service_dir"

  local service_path="$service_dir/displaylink-driver.service"
  cat > "$service_path" <<EOF
[Unit]
Description=DisplayLink Driver Service
After=display-manager.service
Conflicts=getty@tty7.service

[Service]
ExecStartPre=/bin/sh -c 'modprobe evdi || (dkms install \$(ls -t /usr/src | grep evdi | head -n1  | sed -e "s:-:/:") && modprobe evdi)'
ExecStart=/opt/displaylink/DisplayLinkManager
Restart=always
WorkingDirectory=/opt/displaylink
RestartSec=5

EOF
  chmod 0644 "$service_path"
}

get_runit_sv_dir()
{
  local runit_dir=/etc/runit/sv
  [[ -d $runit_dir ]] || runit_dir=/etc/sv
  echo -n "$runit_dir"
}

get_runit_service_dir()
{
  local service_dir=/service
  local search_service_dir
  search_service_dir=$(pgrep -a runsvdir | sed -nE 's~^.*-P[[:space:]]+([^[:space:]]+).*$~\1~p')

  [[ -n $search_service_dir && -d $search_service_dir ]] \
    && service_dir=$search_service_dir
  echo -n "$service_dir"
}

add_runit_service()
{
  local runit_dir
  runit_dir=$(get_runit_sv_dir)
  runit_dir="${1%/}/${runit_dir#/}"

  local driver_name='displaylink-driver'
  mkdir -p "$runit_dir/$driver_name/log"

  cat > "$runit_dir/$driver_name/run" <<EOF
#!/bin/sh
set -e
cd /opt/displaylink
modprobe evdi || (dkms install "\$(ls -t /usr/src | grep evdi | head -n1  | sed -e "s:-:/:")" && modprobe evdi)
exec /opt/displaylink/DisplayLinkManager
EOF

cat > "$runit_dir/$driver_name/log/run" <<EOF
#!/bin/sh
exec svlogd -tt '$LOGSDIR'
EOF

  chmod -R 0755 "$runit_dir/$driver_name"

  local service_dir
  service_dir=$(get_runit_service_dir)
  service_dir="${1%/}/${service_dir#/}"
  touch "$runit_dir/displaylink-driver/down"
  ln -s "$runit_dir/displaylink-driver" "$service_dir"
}

create_dl_service()
{
  local init=$1
  local displaylink_dir=$2
  local install_prefix=${3:-/}

  case $init in
    upstart|systemd|runit)
      "add_${init}_service" "$install_prefix"
      create_pm_script "$init" "$displaylink_dir"
      link_pm_scripts "$init" "$displaylink_dir"
      ;;
  esac
}

create_pm_script()
{
  local init=$1
  local displaylink_dir=$2
  local suspend="$displaylink_dir/suspend.sh"

  cat > "$suspend" << EOF
#!/usr/bin/env bash
# Copyright (c) 2015 - 2019 DisplayLink (UK) Ltd.

suspend_displaylink-driver()
{
  #flush any bytes in pipe
  while read -n 1 -t 1 SUSPEND_RESULT < /tmp/PmMessagesPort_out; do : ; done;

  #suspend DisplayLinkManager
  echo "S" > /tmp/PmMessagesPort_in

  if [[ -p /tmp/PmMessagesPort_out ]]; then
    #wait until suspend of DisplayLinkManager finish
    read -n 1 -t 10 SUSPEND_RESULT < /tmp/PmMessagesPort_out
  fi
}

resume_displaylink-driver()
{
  #resume DisplayLinkManager
  echo "R" > /tmp/PmMessagesPort_in
}

EOF

  case $init in
    upstart)
      cat >> "$suspend" << 'EOF'
case "$1" in
  thaw)
    resume_displaylink-driver
    ;;
  hibernate)
    suspend_displaylink-driver
    ;;
  suspend)
    suspend_displaylink-driver
    ;;
  resume)
    resume_displaylink-driver
    ;;
esac

EOF
      ;;

    systemd)
      cat >> "$suspend" << 'EOF'
main_systemd()
{
  case "$1/$2" in
  pre/*)
    suspend_displaylink-driver
    ;;
  post/*)
    resume_displaylink-driver
    ;;
  esac
}
main_pm()
{
  case "$1" in
    suspend|hibernate)
      suspend_displaylink-driver
      ;;
    resume|thaw)
      resume_displaylink-driver
      ;;
  esac
  true
}

DIR=$(cd "$(dirname "$0")" && pwd)

if [[ $DIR == *systemd* ]]; then
  main_systemd "$@"
elif [[ $DIR == *pm* ]]; then
  main_pm "$@"
fi

EOF
      ;;

    runit)
      cat >> "$suspend" << 'EOF'
case "$ZZZ_MODE" in
  noop)
    suspend_displaylink-driver
    ;;
  standby)
    suspend_displaylink-driver
    ;;
  suspend)
    suspend_displaylink-driver
    ;;
  hibernate)
    suspend_displaylink-driver
    ;;
  resume)
    resume_displaylink-driver
    ;;
  *)
    echo "Unknown ZZZ_MODE $ZZZ_MODE" >&2
    exit 1
    ;;
esac

EOF
      ;;
  esac

  chmod 0755 "$suspend"
}

link_pm_scripts()
{
  local init=$1
  local displaylink_dir=$2
  local suspend="$displaylink_dir/suspend.sh"

  case $init in
    upstart)
      ln -sf "$suspend" /etc/pm/sleep.d/displaylink.sh
      ;;

    systemd)
      ln -sf "$suspend" /lib/systemd/system-sleep/displaylink.sh
      [[ -d /etc/pm/sleep.d ]] && \
        ln -sf "$suspend" /etc/pm/sleep.d/10_displaylink
      ;;

    runit)
      if [[ -d /etc/zzz.d ]]
      then
        ln -sf "$suspend" /etc/zzz.d/suspend/displaylink.sh
        cat >> /etc/zzz.d/resume/displaylink.sh << EOF
#!/bin/sh
ZZZ_MODE=resume '$suspend'
EOF
        chmod 0755 /etc/zzz.d/resume/displaylink.sh
      fi
      ;;
  esac
}


remove_upstart_service()
{
  local driver_name="displaylink-driver"
  if grep -sqi displaylink /etc/init/dlm.conf; then
    driver_name="dlm"
  fi

  echo "Stopping displaylink-driver upstart job"
  stop "$driver_name"
  rm -f "/etc/init/$driver_name.conf"
}

remove_systemd_service()
{
  local driver_name="displaylink-driver"
  if grep -sqi displaylink /lib/systemd/system/dlm.service; then
    driver_name="dlm"
  fi
  echo "Stopping ${driver_name} systemd service"
  systemctl stop "$driver_name.service"
  systemctl disable "$driver_name.service"
  rm -f "/lib/systemd/system/$driver_name.service"
}

remove_runit_service()
{
  local runit_dir
  runit_dir=$(get_runit_sv_dir)
  local service_dir
  service_dir=$(get_runit_service_dir)
  local driver_name='displaylink-driver'

  echo "Stopping $driver_name runit service"
  sv stop "$driver_name"
  rm -f "$service_dir/$driver_name"
  # shellcheck disable=SC2115
  rm -rf "$runit_dir/$driver_name"
}

remove_pm_scripts()
{
  rm -f /etc/pm/sleep.d/displaylink.sh
  rm -f /etc/pm/sleep.d/10_displaylink
  rm -f /lib/systemd/system-sleep/displaylink.sh
  rm -f /etc/zzz.d/suspend/displaylink.sh /etc/zzz.d/resume/displaylink.sh
}

