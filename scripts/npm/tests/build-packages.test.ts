import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/build-packages.ts");

interface TargetSpec {
	target: string;
	ext: "tar.gz" | "zip";
	binary: string;
}

const targets: TargetSpec[] = [
	{ target: "aarch64-unknown-linux-gnu", ext: "tar.gz", binary: "mdt" },
	{ target: "aarch64-unknown-linux-musl", ext: "tar.gz", binary: "mdt" },
	{ target: "aarch64-apple-darwin", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-unknown-linux-gnu", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-unknown-linux-musl", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-apple-darwin", ext: "tar.gz", binary: "mdt" },
	{ target: "x86_64-pc-windows-msvc", ext: "zip", binary: "mdt.exe" },
	{ target: "aarch64-pc-windows-msvc", ext: "zip", binary: "mdt.exe" },
];

test("build-packages requires the expected command line arguments", () => {
	const result = spawnSync("pnpm", ["tsx", scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		String(result.stderr || ""),
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
			String(result.stderr || ""),
			/missing release asset: mdt-aarch64-unknown-linux-gnu-v1\.2\.3\.tar\.gz/,
		);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});

test("build-packages processes release archives without error", () => {
	// Clean up leftover temp directory from previous runs
	const repoPackagesTmp = join(process.cwd(), "packages", ".tmp");
	rmSync(repoPackagesTmp, { recursive: true, force: true });

	const tempRoot = join(
		tmpdir(),
		`mdt-build-packages-${process.pid}-${Date.now()}`,
	);
	const assetsDir = join(tempRoot, "assets");

	try {
		mkdirSync(assetsDir, { recursive: true });

		for (const { target, ext, binary } of targets) {
			const workDir = join(tempRoot, target);
			mkdirSync(workDir, { recursive: true });

			if (ext === "tar.gz") {
				writeFileSync(join(workDir, binary), "#!/bin/sh\necho fake\n", {
					mode: 0o755,
				});
				const result = spawnSync(
					"tar",
					[
						"-czf",
						join(assetsDir, `mdt-${target}-v1.2.3.tar.gz`),
						"-C",
						workDir,
						binary,
					],
					{ encoding: "utf8" },
				);
				assert.equal(result.status, 0, result.stderr);
			} else {
				writeFileSync(join(workDir, binary), "fake-windows");
				const result = spawnSync(
					"zip",
					["-q", join(assetsDir, `mdt-${target}-v1.2.3.zip`), binary],
					{ encoding: "utf8", cwd: workDir },
				);
				assert.equal(result.status, 0, result.stderr);
			}
		}

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--release-tag", "v1.2.3", "--assets-dir", assetsDir],
			{ encoding: "utf8" },
		);

		// Script should succeed when all assets are present
		assert.equal(result.status, 0, String(result.stderr || result.stdout));
		assert.match(String(result.stdout || ""), /Populated platform binaries/);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
