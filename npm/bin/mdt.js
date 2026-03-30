#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const PLATFORM_PACKAGES = {
	darwin: {
		arm64: ["@ifi/mdt-darwin-arm64"],
		x64: ["@ifi/mdt-darwin-x64"],
	},
	linux: {
		arm64: ["@ifi/mdt-linux-arm64-gnu", "@ifi/mdt-linux-arm64-musl"],
		x64: ["@ifi/mdt-linux-x64-gnu", "@ifi/mdt-linux-x64-musl"],
	},
	win32: {
		arm64: ["@ifi/mdt-win32-arm64-msvc"],
		x64: ["@ifi/mdt-win32-x64-msvc"],
	},
};

function getCandidatePackages() {
	return PLATFORM_PACKAGES[process.platform]?.[process.arch] ?? [];
}

function resolveBinary(pkgName) {
	try {
		const packageJsonPath = require.resolve(`${pkgName}/package.json`);
		const packageDir = path.dirname(packageJsonPath);
		const binaryName = process.platform === "win32" ? "mdt.exe" : "mdt";
		const binaryPath = path.join(packageDir, "bin", binaryName);
		if (fs.existsSync(binaryPath)) {
			return binaryPath;
		}
	} catch {
		// Ignore missing optional dependencies and continue trying candidates.
	}

	return null;
}

function shouldTryNextPackage(result) {
	if (result.error) {
		return true;
	}

	if (result.status !== 127) {
		return false;
	}

	const stderr = result.stderr ?? "";
	return /not found|no such file or directory|exec format error/i.test(stderr);
}

function forwardOutput(result) {
	if (result.stdout) {
		process.stdout.write(result.stdout);
	}
	if (result.stderr) {
		process.stderr.write(result.stderr);
	}
}

function main() {
	const candidates = getCandidatePackages();
	if (candidates.length === 0) {
		console.error(
			`mdt does not currently publish npm binaries for ${process.platform}/${process.arch}. ` +
				"Install from the GitHub release page or with `cargo install mdt_cli` instead.",
		);
		process.exit(1);
	}

	const failures = [];
	for (const pkgName of candidates) {
		const binaryPath = resolveBinary(pkgName);
		if (!binaryPath) {
			continue;
		}

		const result = spawnSync(binaryPath, process.argv.slice(2), {
			encoding: "utf8",
			stdio: ["inherit", "pipe", "pipe"],
			windowsHide: false,
		});

		if (shouldTryNextPackage(result)) {
			const detail = result.error?.message ?? result.stderr?.trim() ??
				"failed to launch";
			failures.push(`${pkgName}: ${detail}`);
			continue;
		}

		forwardOutput(result);
		process.exit(result.status ?? 0);
	}

	console.error(
		"Unable to find a compatible mdt binary in the installed npm packages.",
	);
	console.error(`Tried: ${candidates.join(", ")}`);
	if (failures.length > 0) {
		console.error(failures.join("\n"));
	}
	console.error(
		"Reinstall with `npm install -g @ifi/mdt`, download a binary from GitHub releases, or use `cargo install mdt_cli`.",
	);
	process.exit(1);
}

main();
