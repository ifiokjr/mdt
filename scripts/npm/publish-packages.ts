#!/usr/bin/env node

import { spawnSync, type SpawnSyncReturns } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const PLATFORM_PACKAGE_DIRS = [
	"m-d-t__cli-darwin-arm64",
	"m-d-t__cli-darwin-x64",
	"m-d-t__cli-linux-arm64-gnu",
	"m-d-t__cli-linux-arm64-musl",
	"m-d-t__cli-linux-x64-gnu",
	"m-d-t__cli-linux-x64-musl",
	"m-d-t__cli-win32-x64-msvc",
	"m-d-t__cli-win32-arm64-msvc",
];

export const CLI_PACKAGE_DIR = "m-d-t__cli";

export const TRUSTED_PUBLISHING_REPOSITORY = "ifiokjr/mdt";
export const TRUSTED_PUBLISHING_WORKFLOW = "publish.yml";

export const FORBIDDEN_NPM_TOKEN_ENV_KEYS = [
	"NODE_AUTH_TOKEN",
	"NPM_TOKEN",
	"NPM_AUTH_TOKEN",
	"NPM_CONFIG_TOKEN",
	"NPM_CONFIG__AUTH_TOKEN",
	"npm_config_token",
	"npm_config__authToken",
];

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

export function run(
	command: string,
	args: string[],
	options: {
		stdio?: "pipe" | "inherit";
		cwd?: string;
		env?: NodeJS.ProcessEnv;
	} = {},
): SpawnSyncReturns<string> {
	const result = _spawnSync(command, args, {
		encoding: "utf8",
		stdio: options.stdio ?? "pipe",
		cwd: options.cwd,
		env: options.env,
	});

	if (result.status !== 0) {
		const detail = result.stderr || result.stdout ||
			`exit code ${result.status ?? "unknown"}`;
		throw new Error(`${command} ${args.join(" ")} failed: ${detail}`);
	}

	return result;
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

export function assertTrustedPublishingContext(
	env: NodeJS.ProcessEnv = process.env,
): void {
	const configuredTokenKeys = FORBIDDEN_NPM_TOKEN_ENV_KEYS.filter(
		(key) => env[key],
	);
	if (configuredTokenKeys.length > 0) {
		throw new Error(
			`Refusing to publish npm packages with long-lived npm token environment variables: ${
				configuredTokenKeys.join(", ")
			}. ` +
				"Remove npm token credentials so npm trusted publishing can use GitHub OIDC.",
		);
	}

	const workflowRef = env.GITHUB_WORKFLOW_REF ?? "";
	const expectedWorkflowPath =
		`${TRUSTED_PUBLISHING_REPOSITORY}/.github/workflows/${TRUSTED_PUBLISHING_WORKFLOW}@`;
	const missing: string[] = [];

	if (env.GITHUB_ACTIONS !== "true") {
		missing.push("GITHUB_ACTIONS=true");
	}
	if (env.GITHUB_REPOSITORY !== TRUSTED_PUBLISHING_REPOSITORY) {
		missing.push(`GITHUB_REPOSITORY=${TRUSTED_PUBLISHING_REPOSITORY}`);
	}
	if (!workflowRef.startsWith(expectedWorkflowPath)) {
		missing.push(`GITHUB_WORKFLOW_REF=${expectedWorkflowPath}<ref>`);
	}
	if (!env.ACTIONS_ID_TOKEN_REQUEST_URL) {
		missing.push("ACTIONS_ID_TOKEN_REQUEST_URL");
	}
	if (!env.ACTIONS_ID_TOKEN_REQUEST_TOKEN) {
		missing.push("ACTIONS_ID_TOKEN_REQUEST_TOKEN");
	}

	if (missing.length > 0) {
		throw new Error(
			"Cannot publish npm packages without the trusted-publishing GitHub Actions context. " +
				`Expected repository ${TRUSTED_PUBLISHING_REPOSITORY}, workflow ${TRUSTED_PUBLISHING_WORKFLOW}, environment publisher, and OIDC token permissions. ` +
				`Missing or mismatched: ${missing.join(", ")}.`,
		);
	}
}

export function main(argv: string[] = process.argv.slice(2)): void {
	const args = parseArgs(argv);
	if (!args["packages-dir"]) {
		throw new Error("usage: publish-packages.ts --packages-dir <dir>");
	}

	const packagesDir = resolve(args["packages-dir"]);

	for (const dirName of PLATFORM_PACKAGE_DIRS) {
		const dir = join(packagesDir, dirName);
		const pkg = packageMetadata(dir) as { name: string; version: string };
		if (hasBinary(dir) === false) {
			throw new Error(
				`Cannot populate ${pkg.name}@${pkg.version}: no binary found in ${
					join(dir, "bin")
				}. ` +
					"Run build-packages.ts first to populate platform binaries.",
			);
		}
		console.log(`Populated ${pkg.name}@${pkg.version}`);
	}

	const cliDir = join(packagesDir, CLI_PACKAGE_DIR);
	const cliPkg = packageMetadata(cliDir) as { name: string; version: string };
	if (hasBinary(cliDir) === false) {
		throw new Error(
			`Cannot populate ${cliPkg.name}@${cliPkg.version}: no binary found in ${
				join(cliDir, "bin")
			}. ` +
				"Run build-packages.ts first to populate platform binaries.",
		);
	}
	console.log(`Populated ${cliPkg.name}@${cliPkg.version}`);
}

if (
	process.argv[1] &&
	resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))
) {
	main();
}
