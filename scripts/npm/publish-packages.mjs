#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

function parseArgs(argv) {
	const args = {};
	for (let index = 0; index < argv.length; index += 1) {
		const key = argv[index];
		const value = argv[index + 1];
		if (!key.startsWith("--") || value === undefined) {
			continue;
		}
		args[key.slice(2)] = value;
		index += 1;
	}
	return args;
}

function run(command, args, options = {}) {
	const result = spawnSync(command, args, {
		encoding: "utf8",
		stdio: options.stdio ?? "pipe",
		cwd: options.cwd,
	});
	if (result.status !== 0) {
		const detail = result.stderr || result.stdout ||
			`exit code ${result.status ?? "unknown"}`;
		throw new Error(`${command} ${args.join(" ")} failed: ${detail}`);
	}
	return result;
}

function packageMetadata(dir) {
	return JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
}

function packageExists(name, version) {
	const result = spawnSync("npm", [
		"view",
		`${name}@${version}`,
		"version",
		"--json",
	], {
		encoding: "utf8",
		stdio: "pipe",
	});
	return result.status === 0;
}

function publishPackage(dir) {
	const pkg = packageMetadata(dir);
	if (packageExists(pkg.name, pkg.version)) {
		console.log(`Skipping ${pkg.name}@${pkg.version}; already published.`);
		return;
	}

	console.log(`Publishing ${pkg.name}@${pkg.version}...`);
	run("npm", ["publish", "--access", "public", "--provenance"], {
		cwd: dir,
		stdio: "inherit",
	});
}

function main() {
	const args = parseArgs(process.argv.slice(2));
	const packagesDir = resolve(args["packages-dir"] ?? "");
	if (!args["packages-dir"]) {
		throw new Error("usage: publish-packages.mjs --packages-dir <dir>");
	}

	const platformRoot = join(packagesDir, "platform");
	const platformPackages = readdirSync(platformRoot)
		.sort()
		.map((entry) => join(platformRoot, entry));

	for (const packageDir of platformPackages) {
		publishPackage(packageDir);
	}

	// Publish the skills package (independent of platform binaries).
	const skillsDir = join(packagesDir, "skills");
	if (existsSync(join(skillsDir, "package.json"))) {
		publishPackage(skillsDir);
	}

	publishPackage(join(packagesDir, "root"));
}

main();
