#!/usr/bin/env bash
# Idempotent Cursor cloud-agent bootstrap for mdt.
# Installs multi-user Nix (flakes + nix-command) and devenv from the
# devenv.cachix.org binary cache, puts both on the default PATH, warms the
# devenv shell, then installs repo dependencies (`install:all`).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# shellcheck source=ensure-nix-daemon.sh
source "$(dirname "${BASH_SOURCE[0]}")/ensure-nix-daemon.sh"

if [[ -f /etc/profile.d/nix.sh ]]; then
	# shellcheck disable=SC1091
	. /etc/profile.d/nix.sh
fi

if ! command -v nix >/dev/null 2>&1; then
	curl --fail --location --silent --show-error https://nixos.org/nix/install | sh -s -- --daemon --yes
	# shellcheck disable=SC1091
	. /etc/profile.d/nix.sh
fi

configure_nix
ensure_nix_daemon

if ! command -v devenv >/dev/null 2>&1; then
	nix profile install --accept-flake-config nixpkgs#devenv
fi

publish_tool_path

# Populate the devenv shell profile so later `devenv shell` starts stay fast.
devenv shell -- true

# Match CI: locked cargo binaries and a frozen pnpm install.
export CI=true
devenv shell -- install:all
