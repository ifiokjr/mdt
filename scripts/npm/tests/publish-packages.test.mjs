import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/publish-packages.ts");

function makeTempDir(name) {
	return join(
		tmpdir(),
		`mdt-publish-packages-${name}-${process.pid}-${Date.now()}`,
	);
}

function createPackage(dir, name, version) {
	mkdirSync(dir, { recursive: true });
	writeFileSync(
		join(dir, "package.json"),
		JSON.stringify({ name, version }, null, 2),
	);
}

function createFakeNpm(binDir, publishLogPath) {
	mkdirSync(binDir, { recursive: true });
	const scriptPath = join(binDir, "npm");
	const script = `#!/bin/sh
set -eu
cmd="$1"
shift
case "$cmd" in
  view)
    package_ref="$1"
    version="\${package_ref##*@}"
    if [ "$package_ref" = "@m-d-t/cli-linux-x64-gnu@\${version}" ]; then
      printf '%s\\n' "$version"
      exit 0
    fi
    exit 1
    ;;
  publish)
    printf '%s\\n' "$PWD" >> ${JSON.stringify(publishLogPath)}
    exit 0
    ;;
  *)
    echo "unexpected npm command: $cmd" >&2
    exit 1
    ;;
esac
`;
	writeFileSync(scriptPath, script, { mode: 0o755 });
	return scriptPath;
}

test("publish-packages requires a packages directory argument", () => {
	const result = spawnSync("pnpm", ["tsx", scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		result.stderr,
		/usage: publish-packages\.ts --packages-dir <dir>/,
	);
});

test("publish-packages publishes unpublished packages and skips existing ones", () => {
	const tempRoot = makeTempDir("happy");
	const packagesDir = join(tempRoot, "packages");
	const publishLogPath = join(tempRoot, "publish.log");
	const fakeBinDir = join(tempRoot, "bin");

	try {
		// Create ALL platform packages flat under packagesDir (matches publish-packages.ts PLATFORM_PACKAGE_DIRS)
		const ALL_PLATFORM_PACKAGES = [
			"m-d-t__cli-darwin-arm64",
			"m-d-t__cli-darwin-x64",
			"m-d-t__cli-linux-arm64-gnu",
			"m-d-t__cli-linux-arm64-musl",
			"m-d-t__cli-linux-x64-gnu",
			"m-d-t__cli-linux-x64-musl",
			"m-d-t__cli-win32-arm64-msvc",
			"m-d-t__cli-win32-x64-msvc",
		];
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

		createFakeNpm(fakeBinDir, publishLogPath);

		const result = spawnSync(
			"pnpm",
			["tsx", scriptPath, "--packages-dir", packagesDir],
			{
				cwd: process.cwd(),
				encoding: "utf8",
				env: {
					...process.env,
					PATH: `${fakeBinDir}:${process.env.PATH ?? ""}`,
				},
			},
		);

		assert.equal(result.status, 0, result.stderr || result.stdout);
		// linux-x64-gnu is already published (npm view succeeds) → skipped
		assert.match(result.stdout, /Skipping @m-d-t\/cli-linux-x64-gnu@1\.2\.3/);
		// darwin-arm64 is not published (npm view fails) → published
		assert.match(result.stdout, /Publishing @m-d-t\/cli-darwin-arm64@1\.2\.3/);
		// the root CLI package should also be published
		assert.match(result.stdout, /Publishing @m-d-t\/cli@1\.2\.3/);

		// 8 platform packages total: 7 unpublished + 1 skipped = 7 published
		// Plus 1 CLI package = 8 total published
		const publishedDirs = readFileSync(publishLogPath, "utf8")
			.trim()
			.split("\n")
			.filter(Boolean);
		assert.equal(publishedDirs.length, 8);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});
