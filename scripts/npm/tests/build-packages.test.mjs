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
				join(outDir, "platform", "@m-d-t__cli-linux-x64-gnu", "bin", "mdt"),
			),
		);
		assert.ok(
			existsSync(
				join(
					outDir,
					"platform",
					"@m-d-t__cli-win32-x64-msvc",
					"bin",
					"mdt.exe",
				),
			),
		);

		const rootPackage = JSON.parse(
			readFileSync(join(outDir, "root", "package.json"), "utf8"),
		);
		assert.equal(rootPackage.name, "@m-d-t/cli");
		assert.equal(rootPackage.version, "1.2.3");
		assert.equal(rootPackage.bin.mdt, "bin/mdt.js");
		assert.equal(
			rootPackage.optionalDependencies["@m-d-t/cli-darwin-arm64"],
			"1.2.3",
		);

		const linuxPackage = JSON.parse(
			readFileSync(
				join(outDir, "platform", "@m-d-t__cli-linux-x64-gnu", "package.json"),
				"utf8",
			),
		);
		assert.equal(linuxPackage.name, "@m-d-t/cli-linux-x64-gnu");
		assert.deepEqual(linuxPackage.os, ["linux"]);
		assert.deepEqual(linuxPackage.cpu, ["x64"]);
		assert.deepEqual(linuxPackage.libc, ["glibc"]);

		// Skills package assertions
		assert.ok(
			existsSync(join(outDir, "skills", "skills", "mdt", "SKILL.md")),
			"skills package should contain skills/mdt/SKILL.md",
		);
		assert.ok(
			existsSync(join(outDir, "skills", "skills", "mdt", "REFERENCE.md")),
			"skills package should contain skills/mdt/REFERENCE.md",
		);
		assert.ok(
			existsSync(join(outDir, "skills", "README.md")),
			"skills package should contain README.md",
		);
		assert.ok(
			existsSync(join(outDir, "skills", "LICENSE")),
			"skills package should contain LICENSE",
		);

		const skillsPackage = JSON.parse(
			readFileSync(join(outDir, "skills", "package.json"), "utf8"),
		);
		assert.equal(skillsPackage.name, "@m-d-t/skills");
		assert.equal(skillsPackage.version, "1.2.3");
		assert.ok(
			skillsPackage.keywords.includes("pi-package"),
			"skills package should have pi-package keyword",
		);

		// Summary should include skills package
		const summary = JSON.parse(
			readFileSync(join(outDir, "summary.json"), "utf8"),
		);
		assert.equal(summary.skillsPackage, "@m-d-t/skills");
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});

test("build-packages requires the expected command line arguments", () => {
	const result = spawnSync("node", [scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		result.stderr,
		/usage: build-packages\.mjs --version <x\.y\.z> --release-tag <vX\.Y\.Z>/,
	);
});

test("build-packages reports missing release assets", () => {
	const tempRoot = join(
		tmpdir(),
		`mdt-build-packages-missing-${process.pid}-${Date.now()}`,
	);
	const assetsDir = join(tempRoot, "assets");
	const outDir = join(tempRoot, "out");
	mkdirSync(assetsDir, { recursive: true });

	const result = spawnSync(
		"node",
		[
			scriptPath,
			"--version",
			"1.2.3",
			"--release-tag",
			"v1.2.3",
			"--assets-dir",
			assetsDir,
			"--out-dir",
			outDir,
		],
		{ encoding: "utf8" },
	);

	try {
		assert.notEqual(result.status, 0);
		assert.match(
			result.stderr,
			/missing release asset: mdt-aarch64-unknown-linux-gnu-v1.2.3.tar.gz/,
		);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
