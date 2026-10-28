import assert from "node:assert/strict";
import * as path from "node:path";
import { test } from "node:test";
import {
  bundledServerPath,
  pathCandidates,
  resolveServer,
  type ResolveServerOptions,
} from "../src/serverPath.ts";

const extensionRoot = path.resolve("virtual-extension");
const workspaceRoot = path.resolve("virtual-workspace");

function options(
  overrides: Partial<ResolveServerOptions> = {},
): ResolveServerOptions {
  return {
    extensionRoot,
    workspaceRoot,
    platform: "linux",
    arch: "x64",
    pathValue: "",
    isRunnable: () => false,
    ...overrides,
  };
}

test("setting wins and relative paths resolve against the workspace", () => {
  const configured = path.resolve(workspaceRoot, "tools/eudkit-lsp");
  const environment = path.resolve(workspaceRoot, "env/eudkit-lsp");
  const fromPath = path.resolve("path-bin/eudkit-lsp");
  const bundled = bundledServerPath(extensionRoot, "linux", "x64");
  const runnable = new Set([configured, environment, fromPath, bundled]);

  const result = resolveServer(
    options({
      configuredPath: "tools/eudkit-lsp",
      environmentPath: environment,
      pathValue: path.dirname(fromPath),
      isRunnable: (candidate) => runnable.has(candidate),
    }),
  );

  assert.deepEqual(result, {
    found: true,
    path: configured,
    source: "setting",
  });
});

test("environment, PATH, and bundled binaries are used in order", () => {
  const environment = path.resolve("env/eudkit-lsp");
  const fromPath = path.resolve("path-bin/eudkit-lsp");
  const bundled = bundledServerPath(extensionRoot, "linux", "x64");

  const base = options({
    environmentPath: environment,
    pathValue: path.dirname(fromPath),
  });

  assert.deepEqual(
    resolveServer({
      ...base,
      isRunnable: (candidate) => candidate === environment,
    }),
    { found: true, path: environment, source: "environment" },
  );

  assert.deepEqual(
    resolveServer({
      ...base,
      isRunnable: (candidate) => candidate === fromPath,
    }),
    { found: true, path: fromPath, source: "path" },
  );

  assert.deepEqual(
    resolveServer({
      ...base,
      isRunnable: (candidate) => candidate === bundled,
    }),
    { found: true, path: bundled, source: "bundled" },
  );
});

test("a configured path that is missing falls through to the next source", () => {
  const environment = path.resolve("env/eudkit-lsp");

  const result = resolveServer(
    options({
      configuredPath: "missing/eudkit-lsp",
      environmentPath: environment,
      isRunnable: (candidate) => candidate === environment,
    }),
  );

  assert.deepEqual(result, {
    found: true,
    path: environment,
    source: "environment",
  });
});

test("failure reports every source, including unusable relative paths", () => {
  const result = resolveServer(
    options({
      workspaceRoot: undefined,
      configuredPath: "./eudkit-lsp",
      environmentPath: "./eudkit-lsp",
    }),
  );

  assert.equal(result.found, false);
  if (result.found) {
    return;
  }

  const sources = result.searched.map((attempt) => attempt.source);
  assert.deepEqual(sources, ["setting", "environment", "bundled"]);

  for (const attempt of result.searched) {
    if (attempt.source === "setting" || attempt.source === "environment") {
      assert.equal(
        attempt.reason,
        "relative path requires an open workspace folder",
      );
    }
  }
});

test("bundled path uses the platform and architecture directory", () => {
  assert.equal(
    bundledServerPath(extensionRoot, "win32", "arm64"),
    path.join(extensionRoot, "bin", "win32-arm64", "eudkit-lsp.exe"),
  );

  const bundled = bundledServerPath(extensionRoot, "win32", "arm64");
  const result = resolveServer(
    options({
      platform: "win32",
      arch: "arm64",
      isRunnable: (candidate) => candidate === bundled,
    }),
  );

  assert.deepEqual(result, { found: true, path: bundled, source: "bundled" });
});

test("Windows PATH lookup honors PATHEXT", () => {
  const tools = path.resolve("windows-tools");
  const expected = path.resolve(tools, "eudkit-lsp.EXE");

  assert.deepEqual(
    pathCandidates("eudkit-lsp", "win32", tools, ".EXE;.CMD"),
    [expected, path.resolve(tools, "eudkit-lsp.CMD")],
  );

  const result = resolveServer(
    options({
      platform: "win32",
      pathValue: tools,
      pathExtensions: ".EXE;.CMD",
      isRunnable: (candidate) => candidate === expected,
    }),
  );

  assert.deepEqual(result, { found: true, path: expected, source: "path" });
});
