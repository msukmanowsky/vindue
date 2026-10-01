#!/usr/bin/env bash
# AX-correct dev loop: runs the debug binary under launchd instead of a
# terminal. Terminal-spawned processes present their codesign identity to
# TCC, so the System Settings path record never matches and Accessibility
# grants don't stick; launchd-spawned binaries resolve by path and work.
# See AGENTS.md -> Gotchas. Vite serves the UI on :1420.
set -euo pipefail

LABEL="dev.vindue.ax"
UID_NUM="$(id -u)"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$REPO/src-tauri/target/debug/vindue"
VITE_PID_FILE="/tmp/vindue-vite.pid"
VITE_LOG="/tmp/vindue-vite.log"

ensure_vite() {
  if curl -s --max-time 2 -o /dev/null http://localhost:1420 2>/dev/null; then
    echo "vite already serving :1420"
    return
  fi
  echo "starting vite (log: $VITE_LOG)"
  (cd "$REPO" && nohup npm run dev >"$VITE_LOG" 2>&1 & echo $! >"$VITE_PID_FILE")
  for _ in $(seq 1 24); do
    sleep 1
    if curl -s --max-time 1 -o /dev/null http://localhost:1420 2>/dev/null; then
      echo "vite up"
      return
    fi
  done
  echo "vite failed to come up — check $VITE_LOG" >&2
  exit 1
}

write_plist() {
  cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key><string>$LABEL</string>
	<key>ProgramArguments</key>
	<array><string>$BIN</string></array>
	<key>RunAtLoad</key><true/>
</dict>
</plist>
EOF
}

report() {
  sleep 3
  local trusted
  trusted="$(curl -s --max-time 3 http://127.0.0.1:47725/api/v1/state 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin).get("axTrusted"))' 2>/dev/null || true)"
  if [ -n "$trusted" ]; then
    echo "app up; axTrusted: $trusted"
    if [ "$trusted" != "True" ]; then
      echo "grant it once: System Settings -> Privacy & Security -> Accessibility,"
      echo "remove stale 'vindue' entries, add: $BIN"
    fi
  else
    echo "app not answering on :47725 — inspect: launchctl print gui/$UID_NUM/$LABEL" >&2
  fi
}

case "${1:-up}" in
  up)
    # migrate off the previous ad-hoc label if present
    launchctl bootout "gui/$UID_NUM/dev.vindue.test" 2>/dev/null || true
    rm -f "$HOME/Library/LaunchAgents/dev.vindue.test.plist"
    if [ ! -x "$BIN" ]; then
      echo "debug binary missing — building"
      (cd "$REPO/src-tauri" && cargo build)
    fi
    ensure_vite
    write_plist
    launchctl bootstrap "gui/$UID_NUM" "$PLIST" 2>/dev/null \
      || launchctl kickstart -k "gui/$UID_NUM/$LABEL"
    echo "launchd agent $LABEL running $BIN"
    report
    ;;
  restart)
    (cd "$REPO/src-tauri" && cargo build)
    launchctl kickstart -k "gui/$UID_NUM/$LABEL"
    echo "rebuilt + restarted (new cdhash — one Accessibility re-add may be needed)"
    report
    ;;
  down)
    launchctl bootout "gui/$UID_NUM/$LABEL" 2>/dev/null || true
    rm -f "$PLIST"
    if [ -f "$VITE_PID_FILE" ]; then
      kill "$(cat "$VITE_PID_FILE")" 2>/dev/null || true
      rm -f "$VITE_PID_FILE"
    fi
    echo "torn down (agent + plist removed; vite stopped if this script started it)"
    if [ -d "/Applications/Vindue.app" ]; then
      echo "note: while the dev instance ran it reclaimed the login slot (shared"
      echo "config); launch /Applications/Vindue.app once so logins start the"
      echo "real app again — a dev binary at login shows a blank panel (no vite)."
    fi
    ;;
  *)
    echo "usage: $(basename "$0") [up|restart|down]" >&2
    exit 2
    ;;
esac
