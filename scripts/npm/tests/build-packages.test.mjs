import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
	existsSync,
	mkdirSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/build-packages.mjs");

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

test("build-packages generates root and platform npm packages from release archives", () => {
	const tempRoot = join(
		tmpdir(),
		`mdt-build-packages-${process.pid}-${Date.now()}`,
	);
	const assetsDir = join(tempRoot, "assets");
	const outDir = join(tempRoot, "out");
	const releaseTag = "v1.2.3";

	mkdirSync(assetsDir, { recursive: true });
	for (const spec of targets) {
		writeArchive(assetsDir, releaseTag, spec, tempRoot);
	}

	const result = spawnSync(
		"node",
		[
			scriptPath,
			"--version",
			"1.2.3",
			"--release-tag",
			releaseTag,
			"--assets-dir",
			assetsDir,
			"--out-dir",
			outDir,
		],
		{ encoding: "utf8" },
	);

	try {
		assert.equal(result.status, 0, result.stderr || result.stdout);
		assert.ok(existsSync(join(outDir, "root", "bin", "mdt.js")));
		assert.ok(
			existsSync(
				join(outDir, "platform", "@ifi__mdt-linux-x64-gnu", "bin", "mdt"),
			),
		);
		assert.ok(
			existsSync(
				join(outDir, "platform", "@ifi__mdt-win32-x64-msvc", "bin", "mdt.exe"),
			),
		);

		const rootPackage = JSON.parse(
			readFileSync(join(outDir, "root", "package.json"), "utf8"),
		);
		assert.equal(rootPackage.name, "@ifi/mdt");
		assert.equal(rootPackage.version, "1.2.3");
		assert.equal(rootPackage.bin.mdt, "bin/mdt.js");
		assert.equal(
			rootPackage.optionalDependencies["@ifi/mdt-darwin-arm64"],
			"1.2.3",
		);

		const linuxPackage = JSON.parse(
			readFileSync(
				join(outDir, "platform", "@ifi__mdt-linux-x64-gnu", "package.json"),
				"utf8",
			),
		);
		assert.equal(linuxPackage.name, "@ifi/mdt-linux-x64-gnu");
		assert.deepEqual(linuxPackage.os, ["linux"]);
		assert.deepEqual(linuxPackage.cpu, ["x64"]);
		assert.deepEqual(linuxPackage.libc, ["glibc"]);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
