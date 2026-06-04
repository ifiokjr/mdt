#!/usr/bin/env node

import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import {
	chmodSync,
	copyFileSync,
	existsSync,
	mkdirSync,
	readdirSync,
	readFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../..");

interface PlatformSpec {
	archiveExt: string;
	binaryName: string;
	cpu: string;
	label: string;
	libc?: string;
	os: string;
	packageName: string;
	target: string;
}

const platforms: PlatformSpec[] = [
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "arm64",
		label: "Linux arm64 (glibc)",
		libc: "glibc",
		os: "linux",
		packageName: "@m-d-t/cli-linux-arm64-gnu",
		target: "aarch64-unknown-linux-gnu",
	},
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "arm64",
		label: "Linux arm64 (musl)",
		libc: "musl",
		os: "linux",
		packageName: "@m-d-t/cli-linux-arm64-musl",
		target: "aarch64-unknown-linux-musl",
	},
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "arm64",
		label: "macOS arm64",
		os: "darwin",
		packageName: "@m-d-t/cli-darwin-arm64",
		target: "aarch64-apple-darwin",
	},
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "x64",
		label: "Linux x64 (glibc)",
		libc: "glibc",
		os: "linux",
		packageName: "@m-d-t/cli-linux-x64-gnu",
		target: "x86_64-unknown-linux-gnu",
	},
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "x64",
		label: "Linux x64 (musl)",
		libc: "musl",
		os: "linux",
		packageName: "@m-d-t/cli-linux-x64-musl",
		target: "x86_64-unknown-linux-musl",
	},
	{
		archiveExt: "tar.gz",
		binaryName: "mdt",
		cpu: "x64",
		label: "macOS x64",
		os: "darwin",
		packageName: "@m-d-t/cli-darwin-x64",
		target: "x86_64-apple-darwin",
	},
	{
		archiveExt: "zip",
		binaryName: "mdt.exe",
		cpu: "x64",
		label: "Windows x64",
		os: "win32",
		packageName: "@m-d-t/cli-win32-x64-msvc",
		target: "x86_64-pc-windows-msvc",
	},
	{
		archiveExt: "zip",
		binaryName: "mdt.exe",
		cpu: "arm64",
		label: "Windows arm64",
		os: "win32",
		packageName: "@m-d-t/cli-win32-arm64-msvc",
		target: "aarch64-pc-windows-msvc",
	},
];

export const PLATFORM_PACKAGE_DIRS = platforms.map((spec) =>
	packageNameToDirName(spec.packageName)
);

export const CLI_PACKAGE_DIR = "m-d-t__cli";

let _spawnSync = spawnSync;

export function _setSpawnSync(
	fn: typeof spawnSync,
): void {
	_spawnSync = fn;
}

export function _resetSpawnSync(): void {
	_spawnSync = spawnSync;
}

export function parseArgs(argv: string[]): Record<string, string> {
	const args: Record<string, string> = {};
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

export function ensureDirectory(path: string): void {
	mkdirSync(path, { recursive: true });
}

export function run(
	command: string,
	args: string[],
	options: { stdio?: "pipe" | "inherit"; cwd?: string } = {},
): SpawnSyncReturns<string> {
	const result = _spawnSync(command, args, {
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

export function findArchive(
	assetsDir: string,
	target: string,
	releaseTag: string,
	archiveExt: string,
): string {
	const archiveName = `mdt-${target}-${releaseTag}.${archiveExt}`;
	const archivePath = join(assetsDir, archiveName);
	if (!existsSync(archivePath)) {
		throw new Error(`missing release asset: ${archiveName}`);
	}
	return archivePath;
}

export function* walk(dir: string): Generator<string> {
	const entries = readdirSync(dir, { withFileTypes: true });
	for (const entry of entries) {
		const entryPath = join(dir, entry.name);
		if (entry.isDirectory()) {
			yield* walk(entryPath);
		} else {
			yield entryPath;
		}
	}
}

export function extractArchive(
	archivePath: string,
	destinationDir: string,
): void {
	ensureDirectory(destinationDir);
	if (archivePath.endsWith(".zip")) {
		run("unzip", ["-q", archivePath, "-d", destinationDir]);
		return;
	}
	if (archivePath.endsWith(".tar.gz")) {
		run("tar", ["-xzf", archivePath, "-C", destinationDir]);
		return;
	}
	throw new Error(`unsupported archive: ${basename(archivePath)}`);
}

export function findBinary(
	extractedDir: string,
	binaryName: string,
): string {
	for (const filePath of walk(extractedDir)) {
		if (basename(filePath) === binaryName) {
			return filePath;
		}
	}
	throw new Error(`could not find ${binaryName} in ${extractedDir}`);
}

export function packageNameToDirName(packageName: string): string {
	return packageName.replace("@", "").replace("/", "__");
}

export function packageMetadata(dir: string): Record<string, unknown> {
	return JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
}

export function hasBinary(dir: string): boolean {
	const binDir = join(dir, "bin");
	if (!existsSync(binDir)) {
		return false;
	}

	const entries = readdirSync(binDir);
	return entries.some((entry) => entry.startsWith("mdt"));
}

/**
 * Populate a platform-specific package with binaries from release assets.
 * This writes directly into the in-repo packages/ directory, matching monochange's pattern.
 */
export function populatePlatformPackage({
	packagesDir,
	spec,
	releaseTag,
	assetsDir,
	tmpDir,
}: {
	packagesDir: string;
	spec: PlatformSpec;
	releaseTag: string;
	assetsDir: string;
	tmpDir: string;
}): void {
	const archivePath = findArchive(
		assetsDir,
		spec.target,
		releaseTag,
		spec.archiveExt,
	);
	const extractedDir = join(tmpDir, spec.target);
	const packageDir = join(packagesDir, packageNameToDirName(spec.packageName));
	const binDir = join(packageDir, "bin");

	extractArchive(archivePath, extractedDir);
	const binaryPath = findBinary(extractedDir, spec.binaryName);

	ensureDirectory(binDir);
	copyFileSync(binaryPath, join(binDir, spec.binaryName));
	if (spec.binaryName === "mdt") {
		chmodSync(join(binDir, spec.binaryName), 0o755);
	}
}

export function main(argv: string[] = process.argv.slice(2)): void {
	const args = parseArgs(argv);
	const releaseTag = args["release-tag"];
	const assetsDir = resolve(args["assets-dir"] ?? "");

	if (!releaseTag || !args["assets-dir"]) {
		throw new Error(
			"usage: build-packages.ts --release-tag <vX.Y.Z> --assets-dir <dir>",
		);
	}

	const packagesDir = join(repoRoot, "packages");
	const tmpDir = join(packagesDir, ".tmp");

	for (const spec of platforms) {
		populatePlatformPackage({
			packagesDir,
			spec,
			releaseTag,
			assetsDir,
			tmpDir,
		});
	}

	console.log(
		`Populated platform binaries in ${packagesDir} for ${releaseTag}`,
	);
}

if (
	process.argv[1] &&
	resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))
) {
	main();
}
