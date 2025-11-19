#!/usr/bin/env bash
set -euo pipefail

SESSION="stardrive"
ROOT="$HOME/Documents/STARDRIVE"

tmux set-option -g mouse on

if tmux has-session -t "$SESSION" 2>/dev/null; then
  tmux kill-session -t "$SESSION"
fi

tmux new-session -d -s "$SESSION"
tmux send-keys -t "$SESSION" "cd \"$ROOT\" && ./build_driver.sh" C-m
tmux split-window -h -t "$SESSION"
tmux send-keys -t "$SESSION.1" "cd \"$ROOT\" && tail -n 80 /var/log/displaylink-driver.log" C-m
tmux select-pane -t "$SESSION.0"
tmux attach-session -t "$SESSION"
