#!/usr/bin/env bash
#
# Installs a polkit rule so that Tauri x25 can run `pkexec nethogs` and
# `pkexec kill` without prompting for a password each time. Run once after
# installing Tauri x25.
#
#   sudo ./install-polkit-rule.sh           # uses $SUDO_USER and `which nethogs`
#   sudo ./install-polkit-rule.sh <user>    # explicit user
#
# To revert:
#   sudo rm /etc/polkit-1/rules.d/50-tauri-x25-nethogs.rules

set -euo pipefail

RULE_PATH="/etc/polkit-1/rules.d/50-tauri-x25-nethogs.rules"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "error: must be run as root (use sudo)." >&2
  exit 1
fi

TARGET_USER="${1:-${SUDO_USER:-}}"
if [[ -z "$TARGET_USER" ]]; then
  echo "error: could not detect target user. Pass it explicitly:" >&2
  echo "       sudo $0 <username>" >&2
  exit 1
fi

if ! id "$TARGET_USER" >/dev/null 2>&1; then
  echo "error: user '$TARGET_USER' does not exist." >&2
  exit 1
fi

NETHOGS_BIN="$(command -v nethogs || true)"
if [[ -z "$NETHOGS_BIN" ]]; then
  echo "error: nethogs not found in PATH. Install it first:" >&2
  echo "       sudo apt install nethogs" >&2
  exit 1
fi

KILL_BIN="$(command -v kill || true)"
if [[ -z "$KILL_BIN" ]]; then
  KILL_BIN="/usr/bin/kill"
fi

echo "Installing polkit rule:"
echo "  user:    $TARGET_USER"
echo "  nethogs: $NETHOGS_BIN"
echo "  kill:    $KILL_BIN"
echo "  rule:    $RULE_PATH"

cat > "$RULE_PATH" <<EOF
// Tauri x25 — allow $TARGET_USER to run nethogs and kill via pkexec without
// password. Installed by install-polkit-rule.sh. Remove to revert.
polkit.addRule(function(action, subject) {
    if (action.id != "org.freedesktop.policykit.exec") return;
    if (subject.user != "$TARGET_USER") return;
    var program = action.lookup("program");
    if (program == "$NETHOGS_BIN" || program == "$KILL_BIN") {
        return polkit.Result.YES;
    }
});
EOF

chmod 644 "$RULE_PATH"
chown root:root "$RULE_PATH"

if systemctl is-active --quiet polkit; then
  systemctl reload polkit 2>/dev/null || systemctl restart polkit || true
fi

echo "Done. Open Tauri x25 — nethogs and kill should now run without prompts."
