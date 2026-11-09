#!/usr/bin/env bash
# Sourced by the cloud-agent install and start scripts.
# This VM's init is not systemd, so the official multi-user installer does
# not leave nix-daemon running, and a snapshot never keeps the process.

# Same substituter and key documented by https://devenv.sh/getting-started/
# (`cachix use devenv`). Kept in nix.conf so setup does not need the cachix CLI.
DEVENV_CACHE_HOST="devenv.cachix.org"
DEVENV_CACHE_KEY="devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw="

configure_nix() {
	local file="/etc/nix/nix.conf"
	if [[ -f "$file" ]] &&
		grep -qs 'experimental-features = nix-command flakes' "$file" &&
		grep -qs "$DEVENV_CACHE_HOST" "$file" &&
		grep -qs "$DEVENV_CACHE_KEY" "$file"; then
		return 0
	fi

	sudo tee "$file" >/dev/null <<EOF
build-users-group = nixbld
experimental-features = nix-command flakes
extra-substituters = https://${DEVENV_CACHE_HOST}
extra-trusted-public-keys = ${DEVENV_CACHE_KEY}
trusted-users = root ubuntu
EOF

	# The daemon reads nix.conf when it starts.
	if [[ -S /nix/var/nix/daemon-socket/socket ]]; then
		sudo pkill -x nix-daemon || true
		sudo rm -f /nix/var/nix/daemon-socket/socket
	fi
}

nix_store_ready() {
	nix store info >/dev/null 2>&1
}

ensure_nix_daemon() {
	if nix_store_ready; then
		return 0
	fi

	sudo rm -f /nix/var/nix/daemon-socket/socket
	sudo mkdir -p /nix/var/nix/daemon-socket
	sudo setsid /nix/var/nix/profiles/default/bin/nix-daemon >/tmp/nix-daemon.log 2>&1 </dev/null &

	local _attempt
	for _attempt in $(seq 1 50); do
		if nix_store_ready; then
			return 0
		fi
		sleep 0.2
	done

	echo "nix-daemon failed to start; see /tmp/nix-daemon.log" >&2
	return 1
}

# Login shells source /etc/profile.d/nix.sh. Non-login shells do not, so
# publish nix and devenv on the default PATH (/usr/local/bin).
publish_tool_path() {
	local nix_bin=""
	local devenv_bin="${HOME}/.nix-profile/bin/devenv"

	if [[ -x /nix/var/nix/profiles/default/bin/nix ]]; then
		nix_bin="/nix/var/nix/profiles/default/bin/nix"
	elif [[ -x "${HOME}/.nix-profile/bin/nix" ]]; then
		nix_bin="${HOME}/.nix-profile/bin/nix"
	fi

	if [[ -z "$nix_bin" ]]; then
		echo "nix binary not found to publish on PATH" >&2
		return 1
	fi

	sudo ln -sfn "$nix_bin" /usr/local/bin/nix

	if [[ ! -x "$devenv_bin" ]]; then
		echo "devenv binary not found at $devenv_bin" >&2
		return 1
	fi

	sudo ln -sfn "$devenv_bin" /usr/local/bin/devenv
}
