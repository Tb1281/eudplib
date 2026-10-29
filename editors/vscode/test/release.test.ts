import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { test } from "node:test";
import {
  cachedChecksumPath,
  cachedServerPath,
  cacheDirectory,
  checksumAssetName,
  isVersionCompatible,
  parseChecksum,
  parseServerVersion,
  releaseAssetName,
  releaseAssetUrl,
  sha256Hex,
  verifyChecksum,
  versionCompatibilityError,
} from "../src/release.ts";
import { downloadServerBinary } from "../src/serverDownload.ts";

const CHECKSUM_ABC =
  "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

test("maps every published target to its release asset", () => {
  assert.equal(
    releaseAssetName("win32", "x64"),
    "eudkit-lsp-windows-x86_64.exe",
  );
  assert.equal(releaseAssetName("linux", "x64"), "eudkit-lsp-linux-x86_64");
  assert.equal(releaseAssetName("darwin", "arm64"), "eudkit-lsp-macos-aarch64");
  assert.equal(releaseAssetName("darwin", "x64"), "eudkit-lsp-macos-x86_64");
});

test("unsupported platforms fail with an actionable message", () => {
  assert.throws(
    () => releaseAssetName("linux", "arm64"),
    (error: Error) =>
      error.message.includes("unsupported platform linux/arm64") &&
      error.message.includes("EUDKIT_LSP_PATH"),
  );
  assert.throws(
    () => releaseAssetName("win32", "arm64"),
    /Windows x64, Linux x64, macOS arm64, and macOS x86_64/,
  );
});

test("release URLs use the pinned v-prefixed tag and checksum companion", () => {
  const asset = "eudkit-lsp-linux-x86_64";
  assert.equal(
    releaseAssetUrl("0.1.0", asset),
    "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-linux-x86_64",
  );
  assert.equal(
    releaseAssetUrl("v0.1.0", asset),
    "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-linux-x86_64",
  );
  assert.equal(checksumAssetName(asset), `${asset}.sha256`);
});

test("parses bare hashes and sha256sum output", () => {
  const asset = "eudkit-lsp-linux-x86_64";
  assert.equal(parseChecksum(CHECKSUM_ABC), CHECKSUM_ABC);
  assert.equal(
    parseChecksum(`${CHECKSUM_ABC}  ${asset}`, asset),
    CHECKSUM_ABC,
  );
  assert.equal(
    parseChecksum(`${CHECKSUM_ABC.toUpperCase()} *${asset}`, asset),
    CHECKSUM_ABC,
  );
  assert.equal(
    parseChecksum(`${CHECKSUM_ABC}  ${asset}\n`, asset),
    CHECKSUM_ABC,
  );
});

test("rejects malformed or mismatched checksum files", () => {
  assert.throws(() => parseChecksum(""), /checksum file is empty/);
  assert.throws(() => parseChecksum("not-a-checksum"), /valid SHA-256/);
  assert.throws(() => parseChecksum("a".repeat(63)), /valid SHA-256/);
  assert.throws(() => parseChecksum("g".repeat(64)), /valid SHA-256/);
  assert.throws(
    () =>
      parseChecksum(
        `${CHECKSUM_ABC}  eudkit-lsp-windows-x86_64.exe`,
        "eudkit-lsp-linux-x86_64",
      ),
    /does not match/,
  );
});

test("verifies known SHA-256 vectors and reports mismatches", () => {
  assert.equal(sha256Hex(Buffer.from("abc")), CHECKSUM_ABC);
  verifyChecksum(Buffer.from("abc"), `${CHECKSUM_ABC}  asset`);
  assert.throws(
    () =>
      verifyChecksum(
        Buffer.from("abc"),
        `${"0".repeat(64)}  asset`,
      ),
    /SHA-256 mismatch: expected 0{64}, got ba7816bf/,
  );
});

test("parses the eudkit-lsp --version output", () => {
  assert.equal(parseServerVersion("eudkit-lsp 0.1.0\n"), "0.1.0");
  assert.equal(parseServerVersion("eudkit-lsp v0.1.0\r\n"), "0.1.0");
  assert.equal(parseServerVersion("eudkit-lsp 0.1.0-rc.1\n"), "0.1.0-rc.1");
});

test("rejects unrelated version output", () => {
  assert.throws(() => parseServerVersion(""), /did not report a version/);
  assert.throws(
    () => parseServerVersion("eudkit-lsp\n"),
    /did not report a version/,
  );
  assert.throws(
    () => parseServerVersion("eudkit 0.1.0\n"),
    /did not report a version/,
  );
  assert.throws(
    () => parseServerVersion("eudkit-lsp 0.1\n"),
    /did not report a version/,
  );
});

test("accepts the same major/minor series and rejects others", () => {
  assert.equal(isVersionCompatible("0.1.0", "0.1.0"), true);
  assert.equal(isVersionCompatible("0.1.0", "0.1.9"), true);
  assert.equal(isVersionCompatible("0.1.0", "0.2.0"), false);
  assert.equal(isVersionCompatible("0.1.0", "1.0.0"), false);
  assert.equal(isVersionCompatible("0.1.0", "not-a-version"), false);

  const error = versionCompatibilityError("0.1.0", "0.2.0");
  assert.ok(error);
  assert.match(error, /0\.1\.x accepts only eudkit-lsp 0\.1\.x/);
  assert.match(error, /eudkit-lsp 0\.2\.0/);
  assert.equal(versionCompatibilityError("0.1.0", "0.1.2"), undefined);
});

test("cache paths are scoped by extension version and release asset", () => {
  assert.equal(
    cacheDirectory("/storage", "0.1.0"),
    path.join("/storage", "eudkit-lsp", "0.1.0"),
  );
  assert.equal(
    cachedServerPath("/storage", "0.1.0", "linux", "x64"),
    path.join("/storage", "eudkit-lsp", "0.1.0", "eudkit-lsp-linux-x86_64"),
  );
  assert.equal(
    cachedServerPath("/storage", "0.1.0", "win32", "x64"),
    path.join(
      "/storage",
      "eudkit-lsp",
      "0.1.0",
      "eudkit-lsp-windows-x86_64.exe",
    ),
  );
  assert.equal(
    cachedChecksumPath("/storage", "0.1.0", "darwin", "arm64"),
    path.join(
      "/storage",
      "eudkit-lsp",
      "0.1.0",
      "eudkit-lsp-macos-aarch64.sha256",
    ),
  );
});

async function withTempDir(run: (directory: string) => Promise<void>) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), "eudkit-vscode-"));
  try {
    await run(directory);
  } finally {
    await fs.rm(directory, { recursive: true, force: true });
  }
}

test("downloads, verifies, caches, and probes the matching release binary", async () => {
  await withTempDir(async (storagePath) => {
    const binary = Buffer.from("fake eudkit-lsp binary");
    const asset = "eudkit-lsp-linux-x86_64";
    const checksum = `${sha256Hex(binary)}  ${asset}\n`;
    const requested: string[] = [];
    const logs: string[] = [];

    const result = await downloadServerBinary({
      extensionVersion: "0.1.0",
      platform: "linux",
      arch: "x64",
      storagePath,
      log: (line) => logs.push(line),
      fetchImpl: async (input) => {
        const url = String(input);
        requested.push(url);
        return new Response(url.endsWith(".sha256") ? checksum : binary, {
          status: 200,
        });
      },
      probeVersion: async () => "eudkit-lsp 0.1.0\n",
    });

    assert.deepEqual(result, {
      ok: true,
      path: cachedServerPath(storagePath, "0.1.0", "linux", "x64"),
      version: "0.1.0",
      source: "download",
    });
    assert.deepEqual(requested, [
      "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-linux-x86_64.sha256",
      "https://github.com/Tb1281/eudplib/releases/download/v0.1.0/eudkit-lsp-linux-x86_64",
    ]);
    assert.deepEqual(
      await fs.readFile(
        cachedServerPath(storagePath, "0.1.0", "linux", "x64"),
      ),
      binary,
    );
    assert.equal(
      await fs.readFile(
        cachedChecksumPath(storagePath, "0.1.0", "linux", "x64"),
        "utf8",
      ),
      checksum,
    );
    if (process.platform !== "win32") {
      const mode = (await fs.stat(
        cachedServerPath(storagePath, "0.1.0", "linux", "x64"),
      )).mode;
      assert.notEqual(mode & 0o111, 0, "downloaded server must be executable");
    }
    assert.ok(logs.some((line) => line.includes("Verified SHA-256")));
  });
});

test("a checksum mismatch removes the download and reports the asset URL", async () => {
  await withTempDir(async (storagePath) => {
    const binary = Buffer.from("tampered");
    const asset = "eudkit-lsp-linux-x86_64";

    const result = await downloadServerBinary({
      extensionVersion: "0.1.0",
      platform: "linux",
      arch: "x64",
      storagePath,
      fetchImpl: async (input) => {
        const url = String(input);
        return new Response(
          url.endsWith(".sha256") ? `${"0".repeat(64)}  ${asset}\n` : binary,
          { status: 200 },
        );
      },
      probeVersion: async () => "eudkit-lsp 0.1.0\n",
    });

    assert.equal(result.ok, false);
    if (result.ok) {
      return;
    }
    assert.match(result.error, /SHA-256 mismatch/);
    assert.match(result.error, new RegExp(`download/v0\\.1\\.0/${asset}\\.sha256`));
    await assert.rejects(
      fs.stat(cachedServerPath(storagePath, "0.1.0", "linux", "x64")),
    );
  });
});

test("a missing checksum companion is a hard failure", async () => {
  await withTempDir(async (storagePath) => {
    const result = await downloadServerBinary({
      extensionVersion: "0.1.0",
      platform: "linux",
      arch: "x64",
      storagePath,
      fetchImpl: async () =>
        new Response("not found", { status: 404, statusText: "Not Found" }),
      probeVersion: async () => "eudkit-lsp 0.1.0\n",
    });

    assert.equal(result.ok, false);
    if (result.ok) {
      return;
    }
    assert.match(result.error, /required \.sha256 companion/);
    assert.match(result.error, /HTTP 404/);
  });
});

test("reuses a compatible cached binary without a network request", async () => {
  await withTempDir(async (storagePath) => {
    const target = cachedServerPath(
      storagePath,
      "0.1.0",
      "linux",
      "x64",
    );
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, "cached");

    const result = await downloadServerBinary({
      extensionVersion: "0.1.0",
      platform: "linux",
      arch: "x64",
      storagePath,
      fetchImpl: async () => {
        throw new Error("cached installs must not hit the network");
      },
      probeVersion: async () => "eudkit-lsp 0.1.3\n",
    });

    assert.deepEqual(result, {
      ok: true,
      path: target,
      version: "0.1.3",
      source: "cache",
    });
  });
});

test("an incompatible downloaded binary is rejected and removed", async () => {
  await withTempDir(async (storagePath) => {
    const binary = Buffer.from("wrong series");
    const asset = "eudkit-lsp-linux-x86_64";
    const result = await downloadServerBinary({
      extensionVersion: "0.1.0",
      platform: "linux",
      arch: "x64",
      storagePath,
      fetchImpl: async (input) => {
        const url = String(input);
        return new Response(
          url.endsWith(".sha256")
            ? `${sha256Hex(binary)}  ${asset}\n`
            : binary,
          { status: 200 },
        );
      },
      probeVersion: async () => "eudkit-lsp 0.2.0\n",
    });

    assert.equal(result.ok, false);
    if (result.ok) {
      return;
    }
    assert.match(result.error, /incompatible/);
    await assert.rejects(
      fs.stat(cachedServerPath(storagePath, "0.1.0", "linux", "x64")),
    );
  });
});
