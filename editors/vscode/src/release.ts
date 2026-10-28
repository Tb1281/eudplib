import { createHash } from "node:crypto";
import * as path from "node:path";

/** Repository that publishes eudkit-lsp release assets. */
export const RELEASE_REPOSITORY = "Tb1281/eudplib";
export const SERVER_EXECUTABLE = "eudkit-lsp";

export interface ReleaseTarget {
  readonly platform: NodeJS.Platform;
  readonly arch: string;
  readonly assetName: string;
}

const RELEASE_TARGETS: readonly ReleaseTarget[] = [
  {
    platform: "win32",
    arch: "x64",
    assetName: "eudkit-lsp-windows-x86_64.exe",
  },
  {
    platform: "linux",
    arch: "x64",
    assetName: "eudkit-lsp-linux-x86_64",
  },
  {
    platform: "darwin",
    arch: "arm64",
    assetName: "eudkit-lsp-macos-aarch64",
  },
  {
    platform: "darwin",
    arch: "x64",
    assetName: "eudkit-lsp-macos-x86_64",
  },
];

/** Release asset for a platform/architecture pair, or an actionable error. */
export function releaseAssetName(
  platform: NodeJS.Platform,
  arch: string,
): string {
  const target = RELEASE_TARGETS.find(
    (candidate) => candidate.platform === platform && candidate.arch === arch,
  );
  if (!target) {
    throw new Error(
      `unsupported platform ${platform}/${arch}: eudkit-lsp release assets exist only for ` +
        `Windows x64, Linux x64, macOS arm64, and macOS x86_64. Set "eudkit.lsp.path" or ` +
        `EUDKIT_LSP_PATH to a locally built eudkit-lsp, or install it on PATH.`,
    );
  }
  return target.assetName;
}

export function checksumAssetName(assetName: string): string {
  return `${assetName}.sha256`;
}

/** GitHub release download URL pinned to the extension's package version. */
export function releaseAssetUrl(version: string, assetName: string): string {
  const tag = normalizeVersion(version);
  return `https://github.com/${RELEASE_REPOSITORY}/releases/download/v${tag}/${assetName}`;
}

export function normalizeVersion(version: string): string {
  return version.trim().replace(/^v/, "");
}

/** Writable cache directory for one extension version. */
export function cacheDirectory(
  globalStoragePath: string,
  version: string,
): string {
  return path.join(globalStoragePath, SERVER_EXECUTABLE, normalizeVersion(version));
}

/** Cached executable for one extension version and release asset. */
export function cachedServerPath(
  globalStoragePath: string,
  version: string,
  platform: NodeJS.Platform,
  arch: string,
): string {
  return path.join(
    cacheDirectory(globalStoragePath, version),
    releaseAssetName(platform, arch),
  );
}

/** Cached checksum companion that documents what was verified. */
export function cachedChecksumPath(
  globalStoragePath: string,
  version: string,
  platform: NodeJS.Platform,
  arch: string,
): string {
  return path.join(
    cacheDirectory(globalStoragePath, version),
    checksumAssetName(releaseAssetName(platform, arch)),
  );
}

/**
 * Parses a published SHA-256 companion.
 *
 * Accepts a bare hash and `sha256sum` output (`<hash>  <asset>` or
 * `<hash> *<asset>`). When the checksum file names an asset and an expected
 * name is supplied, the names must match so a checksum for another platform
 * cannot be used accidentally.
 */
export function parseChecksum(
  checksumFile: string,
  expectedAssetName?: string,
): string {
  const firstLine = checksumFile
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0);

  if (!firstLine) {
    throw new Error("checksum file is empty");
  }

  const match = /^([0-9a-fA-F]{64})(?:\s+\*?(.+))?$/.exec(firstLine);
  if (!match) {
    throw new Error(
      `checksum file does not contain a valid SHA-256 hash: ${firstLine}`,
    );
  }

  const checksum = match[1].toLowerCase();
  const namedAsset = match[2]?.trim();
  if (
    expectedAssetName &&
    namedAsset &&
    path.basename(namedAsset) !== path.basename(expectedAssetName)
  ) {
    throw new Error(
      `checksum file names ${namedAsset}, which does not match ${expectedAssetName}`,
    );
  }
  return checksum;
}

export function sha256Hex(contents: Uint8Array | string): string {
  return createHash("sha256").update(contents).digest("hex");
}

/** Throws when `contents` do not match the published checksum companion. */
export function verifyChecksum(
  contents: Uint8Array | string,
  checksumFile: string,
  expectedAssetName?: string,
): void {
  const expected = parseChecksum(checksumFile, expectedAssetName);
  const actual = sha256Hex(contents);
  if (actual !== expected) {
    throw new Error(`SHA-256 mismatch: expected ${expected}, got ${actual}`);
  }
}

/**
 * Parses the output of `eudkit-lsp --version`.
 *
 * The server prints exactly `eudkit-lsp <version>`; anything else means the
 * binary is not the expected language server.
 */
export function parseServerVersion(output: string): string {
  const firstLine = output.trim().split(/\r?\n/, 1)[0] ?? "";
  const match =
    /^eudkit-lsp\s+v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$/.exec(
      firstLine.trim(),
    );
  if (!match) {
    throw new Error(
      `eudkit-lsp did not report a version; expected "eudkit-lsp <version>", got ${JSON.stringify(
        firstLine,
      )}`,
    );
  }
  return match[1];
}

interface ParsedVersion {
  readonly major: number;
  readonly minor: number;
}

function parseVersionBase(version: string): ParsedVersion | undefined {
  const match = /^v?(\d+)\.(\d+)\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.exec(
    version.trim(),
  );
  if (!match) {
    return undefined;
  }
  return { major: Number(match[1]), minor: Number(match[2]) };
}

/**
 * Version compatibility rule: extension `0.1.x` accepts only eudkit-lsp
 * `0.1.x`. Patch releases stay compatible within a series.
 */
export function isVersionCompatible(
  extensionVersion: string,
  serverVersion: string,
): boolean {
  const extension = parseVersionBase(extensionVersion);
  const server = parseVersionBase(serverVersion);
  return (
    extension !== undefined &&
    server !== undefined &&
    extension.major === server.major &&
    extension.minor === server.minor
  );
}

export function versionCompatibilityError(
  extensionVersion: string,
  serverVersion: string,
): string | undefined {
  if (isVersionCompatible(extensionVersion, serverVersion)) {
    return undefined;
  }

  const extensionBase = parseVersionBase(extensionVersion);
  const series = extensionBase
    ? `${extensionBase.major}.${extensionBase.minor}`
    : extensionVersion;
  return (
    `eudkit-lsp ${serverVersion} is incompatible with eudkit-vscode ${extensionVersion}: ` +
    `extension ${series}.x accepts only eudkit-lsp ${series}.x. Install the matching ` +
    `eudkit-lsp release (v${normalizeVersion(extensionVersion)}) or update the extension.`
  );
}
