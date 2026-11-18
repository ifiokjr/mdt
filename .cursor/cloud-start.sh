#!/usr/bin/env bash
# nix-daemon does not survive a VM snapshot, and this image has no systemd.
# Install lives in cloud-install.sh.
set -euo pipefail

if [[ -f /etc/profile.d/nix.sh ]]; then
	# shellcheck disable=SC1091
	. /etc/profile.d/nix.sh
fi

# shellcheck source=ensure-nix-daemon.sh
source "$(dirname "${BASH_SOURCE[0]}")/ensure-nix-daemon.sh"
ensure_nix_daemon
publish_tool_path
