#!/usr/bin/env node
"use strict";

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";

const require = createRequire(import.meta.url);

const PLATFORM_PACKAGES: Record<string, Record<string, string[]>> = {
	darwin: {
		arm64: ["@m-d-t/cli-darwin-arm64"],
		x64: ["@m-d-t/cli-darwin-x64"],
	},
	linux: {
		arm64: ["@m-d-t/cli-linux-arm64-gnu", "@m-d-t/cli-linux-arm64-musl"],
		x64: ["@m-d-t/cli-linux-x64-gnu", "@m-d-t/cli-linux-x64-musl"],
	},
	win32: {
		arm64: ["@m-d-t/cli-win32-arm64-msvc"],
		x64: ["@m-d-t/cli-win32-x64-msvc"],
	},
};

function getCandidatePackages(): string[] {
	return PLATFORM_PACKAGES[process.platform]?.[process.arch] ?? [];
}

function resolveBinary(pkgName: string): string | null {
	try {
		const packageJsonPath = require.resolve(`${pkgName}/package.json`);
		const packageDir = dirname(packageJsonPath);
		const binaryName = process.platform === "win32" ? "mdt.exe" : "mdt";
		const binaryPath = join(packageDir, "bin", binaryName);
		if (existsSync(binaryPath)) {
			return binaryPath;
		}
	} catch {
		// Ignore missing optional dependencies and continue trying candidates.
	}

	return null;
}

function shouldTryNextPackage(result: ReturnType<typeof spawnSync>): boolean {
	if (result.error) {
		return true;
	}

	if (result.status !== 127) {
		return false;
	}

	const stderr = String(result.stderr ?? "");
	return /not found|no such file or directory|exec format error/i.test(stderr);
}

function forwardOutput(result: ReturnType<typeof spawnSync>): void {
	if (result.stdout) {
		process.stdout.write(String(result.stdout));
	}
	if (result.stderr) {
		process.stderr.write(String(result.stderr));
	}
}

function main(): void {
	const candidates = getCandidatePackages();
	if (candidates.length === 0) {
		console.error(
			`mdt does not currently publish npm binaries for ${process.platform}/${process.arch}. ` +
				"Install from the GitHub release page or with `cargo install mdt_cli` instead.",
		);
		process.exit(1);
	}

	const failures: string[] = [];
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
		"Reinstall with `npm install -g @m-d-t/cli`, download a binary from GitHub releases, or use `cargo install mdt_cli`.",
	);
	process.exit(1);
}

main();
