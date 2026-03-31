import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const scriptPath = join(process.cwd(), "scripts/npm/publish-packages.mjs");

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
    if [ "$package_ref" = "@ifi/mdt-linux-x64-gnu@\${version}" ]; then
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

test("publish-packages publishes unpublished packages and skips existing ones", () => {
	const tempRoot = makeTempDir("happy");
	const packagesDir = join(tempRoot, "packages");
	const platformDir = join(packagesDir, "platform");
	const rootDir = join(packagesDir, "root");
	const publishLogPath = join(tempRoot, "publish.log");
	const fakeBinDir = join(tempRoot, "bin");

	try {
		createPackage(
			join(platformDir, "@ifi__mdt-darwin-arm64"),
			"@ifi/mdt-darwin-arm64",
			"1.2.3",
		);
		createPackage(
			join(platformDir, "@ifi__mdt-linux-x64-gnu"),
			"@ifi/mdt-linux-x64-gnu",
			"1.2.3",
		);
		createPackage(rootDir, "@ifi/mdt", "1.2.3");
		createFakeNpm(fakeBinDir, publishLogPath);

		const result = spawnSync(
			"node",
			[scriptPath, "--packages-dir", packagesDir],
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
		assert.match(result.stdout, /Skipping @ifi\/mdt-linux-x64-gnu@1.2.3/);
		assert.match(result.stdout, /Publishing @ifi\/mdt-darwin-arm64@1.2.3/);
		assert.match(result.stdout, /Publishing @ifi\/mdt@1.2.3/);

		const publishedDirs = readFileSync(publishLogPath, "utf8")
			.trim()
			.split("\n")
			.filter(Boolean);
		assert.equal(publishedDirs.length, 2);
		assert.match(
			publishedDirs[0],
			/packages\/platform\/@ifi__mdt-darwin-arm64$/,
		);
		assert.match(publishedDirs[1], /packages\/root$/);
	} finally {
		rmSync(tempRoot, { recursive: true, force: true });
	}
});

test("publish-packages requires a packages directory argument", () => {
	const result = spawnSync("node", [scriptPath], {
		cwd: process.cwd(),
		encoding: "utf8",
	});
	assert.notEqual(result.status, 0);
	assert.match(
		result.stderr,
		/usage: publish-packages\.mjs --packages-dir <dir>/,
	);
});
