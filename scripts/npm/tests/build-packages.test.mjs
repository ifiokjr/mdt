import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/build-packages.ts");

const targets = [
	{ target: "aarch64-unknown-linux-gnu", ext: "tar.gz", binary: "mdt" },
	{ target: "aarch64-unknown-linux-musl", ext: "tar.gz", binary: "mdt" },
	{ target: "aarch64-apple-darwin", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-unknown-linux-gnu", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-unknown-linux-musl", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-apple-darwin", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-pc-windows-msvc", ext: "zip", binary: "mdt.exe" },
	{ target: "aarch64-pc-windows-msvc", ext: "zip", binary: "mdt.exe" },
];

function run(command, args, cwd) {
	const result = spawnSync(command, args, { cwd, encoding: "utf8" });
	assert.equal(result.status, 0, result.stderr || result.stdout);
}

function writeArchive(assetsDir, releaseTag, spec, tempRoot) {
	const workDir = join(tempRoot, spec.target);
	mkdirSync(workDir, { recursive: true });
	if (spec.binary === "mdt") {
		writeFileSync(join(workDir, spec.binary), "#!/bin/sh\necho packaged\n", {
			mode: 0o755,
		});
		run(
			"tar",
			[
				"-czf",
				join(assetsDir, `mdt-${spec.target}-${releaseTag}.tar.gz`),
				"-C",
				workDir,
				spec.binary,
			],
			process.cwd(),
		);
	} else {
		writeFileSync(join(workDir, spec.binary), "fake-windows-binary");
		run(
			"zip",
			[
				"-q",
				join(assetsDir, `mdt-${spec.target}-${releaseTag}.zip`),
				spec.binary,
			],
			workDir,
		);
	}
}

test("build-packages populates platform packages from release archives", () => {
	const tempRoot = join(
		tmpdir(),
		`mdt-build-packages-${process.pid}-${Date.now()}`,
	);
	const assetsDir = join(tempRoot, "assets");

	try {
		mkdirSync(assetsDir, { recursive: true });
		for (const spec of targets) {
			writeArchive(assetsDir, "v1.2.3", spec, tempRoot);
		}

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--release-tag", "v1.2.3", "--assets-dir", assetsDir],
			{ encoding: "utf8" },
		);

		assert.equal(result.status, 0, result.stderr || result.stdout);
		assert.match(
			result.stdout,
			/Populated platform binaries/,
		);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});

test("build-packages requires the expected command line arguments", () => {
	const result = spawnSync("pnpm", ["tsx", scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		result.stderr,
		/usage: build-packages\.ts --release-tag <vX\.Y\.Z> --assets-dir <dir>/,
	);
});

test("build-packages reports missing release assets", () => {
	const tempRoot = join(
		tmpdir(),
		`mdt-build-packages-missing-${process.pid}-${Date.now()}`,
	);
	const assetsDir = join(tempRoot, "assets");

	try {
		mkdirSync(assetsDir, { recursive: true });

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--release-tag", "v1.2.3", "--assets-dir", assetsDir],
			{ encoding: "utf8" },
		);

		assert.notEqual(result.status, 0);
		assert.match(
			result.stderr,
			/missing release asset: mdt-aarch64-unknown-linux-gnu-v1\.2\.3\.tar\.gz/,
		);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
