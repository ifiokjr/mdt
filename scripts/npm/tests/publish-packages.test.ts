import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/publish-packages.ts");

const ALL_PLATFORM_PACKAGES: string[] = [
	"m-d-t__cli-darwin-arm64",
	"m-d-t__cli-darwin-x64",
	"m-d-t__cli-linux-arm64-gnu",
	"m-d-t__cli-linux-arm64-musl",
	"m-d-t__cli-linux-x64-gnu",
	"m-d-t__cli-linux-x64-musl",
	"m-d-t__cli-win32-arm64-msvc",
	"m-d-t__cli-win32-x64-msvc",
];

function makeTempDir(name: string): string {
	return join(
		tmpdir(),
		`mdt-publish-packages-${name}-${process.pid}-${Date.now()}`,
	);
}

function createPackage(dir: string, name: string, version: string): void {
	mkdirSync(dir, { recursive: true });
	writeFileSync(
		join(dir, "package.json"),
		JSON.stringify({ name, version }, null, 2),
	);
}

test("publish-packages requires a packages directory argument", () => {
	const result = spawnSync("pnpm", ["tsx", scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		String(result.stderr || result.stdout || ""),
		/usage: publish-packages\.ts --packages-dir <dir>/,
	);
});

test("publish-packages validates packages have binaries", () => {
	const tempRoot = makeTempDir("happy");
	const packagesDir = join(tempRoot, "packages");

	try {
		// Create ALL platform packages flat under packagesDir
		for (const dirName of ALL_PLATFORM_PACKAGES) {
			const pkgDir = join(packagesDir, dirName);
			const pkgName = `@m-d-t/${dirName.replace("m-d-t__cli-", "cli-")}`;
			createPackage(pkgDir, pkgName, "1.2.3");
			// Create a fake binary so hasBinary() passes
			mkdirSync(join(pkgDir, "bin"), { recursive: true });
			writeFileSync(join(pkgDir, "bin", "mdt"), "fake", { mode: 0o755 });
		}

		// Create the root CLI package
		const cliDir = join(packagesDir, "m-d-t__cli");
		createPackage(cliDir, "@m-d-t/cli", "1.2.3");
		mkdirSync(join(cliDir, "bin"), { recursive: true });
		writeFileSync(join(cliDir, "bin", "mdt.js"), "fake launcher", {
			mode: 0o755,
		});

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--packages-dir", packagesDir],
			{ encoding: "utf8" },
		);

		const output = String(result.stdout || "");
		assert.equal(result.status, 0, String(result.stderr || output));
		assert.match(output, /Populated @m-d-t\/cli-linux-x64-gnu@1\.2\.3/);
		assert.match(output, /Populated @m-d-t\/cli-darwin-arm64@1\.2\.3/);
		assert.match(output, /Populated @m-d-t\/cli@1\.2\.3/);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});

test("publish-packages errors when binaries are missing", () => {
	const tempRoot = makeTempDir("missing-binary");
	const packagesDir = join(tempRoot, "packages");

	try {
		// Create only one platform package WITHOUT a binary
		const pkgDir = join(packagesDir, "m-d-t__cli-darwin-arm64");
		createPackage(pkgDir, "@m-d-t/cli-darwin-arm64", "1.2.3");
		// No bin/ directory created

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--packages-dir", packagesDir],
			{ encoding: "utf8" },
		);

		const errOutput = String(result.stderr || result.stdout || "");
		assert.notEqual(result.status, 0);
		assert.match(errOutput, /Cannot populate @m-d-t\/cli-darwin-arm64@1\.2\.3/);
		assert.match(errOutput, /no binary found/);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
