import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const launcherPath = join(process.cwd(), "npm/bin/mdt.js");

const platformPackages = {
	darwin: {
		arm64: ["@ifi/mdt-darwin-arm64"],
		x64: ["@ifi/mdt-darwin-x64"],
	},
	linux: {
		arm64: ["@ifi/mdt-linux-arm64-gnu", "@ifi/mdt-linux-arm64-musl"],
		x64: ["@ifi/mdt-linux-x64-gnu", "@ifi/mdt-linux-x64-musl"],
	},
	win32: {
		arm64: ["@ifi/mdt-win32-arm64-msvc"],
		x64: ["@ifi/mdt-win32-x64-msvc"],
	},
};

function makeTempDir(name) {
	return join(tmpdir(), `mdt-npm-test-${name}-${process.pid}-${Date.now()}`);
}

function setupNodePath(name) {
	const root = makeTempDir(name);
	const nodeModulesDir = join(root, "node_modules");
	mkdirSync(nodeModulesDir, { recursive: true });
	return { root, nodeModulesDir };
}

function createPackage(nodeModulesDir, pkgName, binaryContent) {
	const packageDir = join(nodeModulesDir, ...pkgName.split("/"));
	const binDir = join(packageDir, "bin");
	mkdirSync(binDir, { recursive: true });
	writeFileSync(
		join(packageDir, "package.json"),
		JSON.stringify({ name: pkgName, version: "0.0.0" }, null, 2),
	);

	if (process.platform === "win32") {
		writeFileSync(join(binDir, "mdt.exe"), binaryContent);
	} else {
		writeFileSync(join(binDir, "mdt"), binaryContent, { mode: 0o755 });
	}
}

function runLauncher(nodeModulesDir, args) {
	return spawnSync("node", [launcherPath, ...args], {
		cwd: process.cwd(),
		encoding: "utf8",
		env: {
			...process.env,
			NODE_PATH: nodeModulesDir,
		},
	});
}

function currentCandidates() {
	return platformPackages[process.platform]?.[process.arch] ?? [];
}

test("launcher executes the installed platform binary", () => {
	const { root, nodeModulesDir } = setupNodePath("run");
	try {
		const [pkgName] = currentCandidates();
		assert.ok(pkgName, "expected a package mapping for the current platform");
		const binary = process.platform === "win32"
			? "@echo off\r\necho launcher-ok %*\r\n"
			: '#!/bin/sh\necho launcher-ok "$@"\n';
		createPackage(nodeModulesDir, pkgName, binary);

		const result = runLauncher(nodeModulesDir, ["check", "--verbose"]);
		assert.equal(result.status, 0, result.stderr);
		assert.match(result.stdout, /launcher-ok/);
		assert.match(result.stdout, /check/);
		assert.match(result.stdout, /verbose/);
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test("launcher shows a helpful error when no platform package is installed", () => {
	const { root, nodeModulesDir } = setupNodePath("missing");
	try {
		const result = runLauncher(nodeModulesDir, ["--help"]);
		assert.notEqual(result.status, 0);
		assert.match(result.stderr, /Unable to find a compatible mdt binary/);
		assert.match(result.stderr, /Reinstall with `npm install -g @ifi\/mdt`/);
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test("launcher reports unsupported platforms before attempting resolution", {
	skip: currentCandidates().length === 0,
}, () => {
	const { root, nodeModulesDir } = setupNodePath("unsupported");
	try {
		const result = spawnSync(
			"node",
			[
				"-e",
				`Object.defineProperty(process, "platform", { value: "sunos" }); Object.defineProperty(process, "arch", { value: "x64" }); require(${
					JSON.stringify(launcherPath)
				});`,
			],
			{
				cwd: process.cwd(),
				encoding: "utf8",
				env: {
					...process.env,
					NODE_PATH: nodeModulesDir,
				},
			},
		);
		assert.notEqual(result.status, 0);
		assert.match(result.stderr, /does not currently publish npm binaries/);
		assert.match(result.stderr, /sunos\/x64/);
	} finally {
		rmSync(root, { recursive: true, force: true });
	}
});

test(
	"launcher falls back to the secondary linux package when the first one fails to launch",
	{
		skip: process.platform !== "linux" || currentCandidates().length < 2,
	},
	() => {
		const { root, nodeModulesDir } = setupNodePath("fallback");
		try {
			const [firstPackage, secondPackage] = currentCandidates();
			createPackage(nodeModulesDir, firstPackage, "#!/missing/interpreter\n");
			createPackage(
				nodeModulesDir,
				secondPackage,
				'#!/bin/sh\necho fallback-ok "$@"\n',
			);

			const result = runLauncher(nodeModulesDir, ["doctor"]);
			assert.equal(result.status, 0, result.stderr);
			assert.match(result.stdout, /fallback-ok/);
			assert.match(result.stdout, /doctor/);
		} finally {
			rmSync(root, { recursive: true, force: true });
		}
	},
);
