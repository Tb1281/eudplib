import { execFile } from "node:child_process";
import { promises as fs } from "node:fs";
import {
  cachedChecksumPath,
  cachedServerPath,
  cacheDirectory,
  checksumAssetName,
  parseServerVersion,
  releaseAssetName,
  releaseAssetUrl,
  verifyChecksum,
  versionCompatibilityError,
} from "./release";

const DOWNLOAD_TIMEOUT_MS = 120_000;
const VERSION_TIMEOUT_MS = 10_000;

export type DownloadSource = "download" | "cache";

export type DownloadResult =
  | {
      readonly ok: true;
      readonly path: string;
      readonly version: string;
      readonly source: DownloadSource;
    }
  | { readonly ok: false; readonly error: string };

export interface DownloadServerOptions {
  /** Extension package version; also pins the GitHub release tag. */
  readonly extensionVersion: string;
  /** VS Code global storage directory (context.globalStorageUri.fsPath). */
  readonly storagePath: string;
  readonly platform?: NodeJS.Platform;
  readonly arch?: string;
  readonly fetchImpl?: typeof fetch;
  readonly log?: (line: string) => void;
  /** Injectable version probe for tests. */
  readonly probeVersion?: (binaryPath: string) => Promise<string>;
}

/**
 * Ensures a checksum-verified eudkit-lsp binary exists in the extension cache.
 *
 * The download is pinned to the extension's own version (`v0.1.0` for an
 * `0.1.0` extension), the published `.sha256` companion is mandatory, and the
 * binary must answer `--version` with a compatible `0.1.x` version.
 */
export async function downloadServerBinary(
  options: DownloadServerOptions,
): Promise<DownloadResult> {
  const platform = options.platform ?? process.platform;
  const arch = options.arch ?? process.arch;
  const log = options.log ?? (() => {});
  const fetchImpl = options.fetchImpl ?? fetch;
  const probeVersion = options.probeVersion ?? probeServerVersion;

  let assetName: string;
  try {
    assetName = releaseAssetName(platform, arch);
  } catch (error) {
    return failed(error);
  }

  const version = options.extensionVersion.trim().replace(/^v/, "");
  const cacheDir = cacheDirectory(options.storagePath, version);
  const serverPath = cachedServerPath(
    options.storagePath,
    version,
    platform,
    arch,
  );
  const checksumPath = cachedChecksumPath(
    options.storagePath,
    version,
    platform,
    arch,
  );

  if (await isFile(serverPath)) {
    log(`Found cached eudkit-lsp at ${serverPath}; verifying its version.`);
    const cachedVersion = await checkServerVersion(
      serverPath,
      options.extensionVersion,
      probeVersion,
    );
    if (cachedVersion.ok) {
      log(`Using cached eudkit-lsp ${cachedVersion.version} at ${serverPath}.`);
      return {
        ok: true,
        path: serverPath,
        version: cachedVersion.version,
        source: "cache",
      };
    }

    log(
      `Discarding cached eudkit-lsp at ${serverPath}: ${cachedVersion.error}`,
    );
    await removeFile(serverPath);
    await removeFile(checksumPath);
  }

  const checksumUrl = releaseAssetUrl(
    version,
    checksumAssetName(assetName),
  );
  const downloadUrl = releaseAssetUrl(version, assetName);
  let temporaryPath: string | undefined;

  try {
    log(`Downloading required checksum ${checksumUrl}`);
    const checksumText = await fetchText(
      checksumUrl,
      fetchImpl,
      "required .sha256 companion",
    );

    log(`Downloading ${downloadUrl}`);
    const binary = await fetchBytes(downloadUrl, fetchImpl);
    try {
      verifyChecksum(binary, checksumText, assetName);
    } catch (error) {
      throw new Error(
        `Refusing to install ${assetName}: ${asMessage(error)} (published at ${checksumUrl}).`,
      );
    }
    log(`Verified SHA-256 checksum for ${assetName}.`);

    await fs.mkdir(cacheDir, { recursive: true });
    temporaryPath = `${serverPath}.download`;
    await removeFile(temporaryPath);
    await fs.writeFile(temporaryPath, binary, { mode: 0o755 });
    if (platform !== "win32") {
      await fs.chmod(temporaryPath, 0o755);
    }
    await removeFile(serverPath);
    await fs.rename(temporaryPath, serverPath);
    temporaryPath = undefined;
    await fs.writeFile(checksumPath, checksumText, "utf8");

    const installed = await checkServerVersion(
      serverPath,
      options.extensionVersion,
      probeVersion,
    );
    if (!installed.ok) {
      await removeFile(serverPath);
      await removeFile(checksumPath);
      return failed(
        `${installed.error} Removed the downloaded binary; reinstall the matching ` +
          `extension/server release or set "eudkit.lsp.path" to a compatible eudkit-lsp.`,
      );
    }

    log(
      `Installed eudkit-lsp ${installed.version} at ${serverPath} (checksum ${checksumUrl}).`,
    );
    return {
      ok: true,
      path: serverPath,
      version: installed.version,
      source: "download",
    };
  } catch (error) {
    if (temporaryPath) {
      await removeFile(temporaryPath);
    }
    return failed(
      `Could not install ${assetName}: ${asMessage(error)} ` +
        `Set "eudkit.lsp.path" or EUDKIT_LSP_PATH to a compatible local binary, or check ` +
        `your network connection and retry.`,
    );
  }
}

/**
 * Probes a binary's `--version` output and enforces the extension/server
 * compatibility rule. Shared by downloads and locally resolved binaries.
 */
export async function checkServerVersion(
  binaryPath: string,
  extensionVersion: string,
  probeVersion: (binaryPath: string) => Promise<string> = probeServerVersion,
): Promise<
  | { readonly ok: true; readonly version: string }
  | { readonly ok: false; readonly error: string }
> {
  let output: string;
  try {
    output = await probeVersion(binaryPath);
  } catch (error) {
    return {
      ok: false,
      error: `Could not run "${binaryPath} --version": ${asMessage(error)}.`,
    };
  }

  let version: string;
  try {
    version = parseServerVersion(output);
  } catch (error) {
    return { ok: false, error: asMessage(error) };
  }

  const incompatibility = versionCompatibilityError(extensionVersion, version);
  if (incompatibility) {
    return { ok: false, error: incompatibility };
  }
  return { ok: true, version };
}

/** Runs `eudkit-lsp --version` and returns its output. */
export function probeServerVersion(binaryPath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    execFile(
      binaryPath,
      ["--version"],
      { timeout: VERSION_TIMEOUT_MS, windowsHide: true },
      (error, stdout, stderr) => {
        if (error) {
          const details = stderr.trim() || error.message;
          reject(new Error(details));
          return;
        }
        resolve(stdout);
      },
    );
  });
}

async function fetchText(
  url: string,
  fetchImpl: typeof fetch,
  label: string,
): Promise<string> {
  const response = await fetchResponse(url, fetchImpl, label);
  const text = await response.text();
  if (!text.trim()) {
    throw new Error(`${label} at ${url} is empty.`);
  }
  return text;
}

async function fetchBytes(url: string, fetchImpl: typeof fetch): Promise<Buffer> {
  const response = await fetchResponse(url, fetchImpl, "release asset");
  return Buffer.from(await response.arrayBuffer());
}

async function fetchResponse(
  url: string,
  fetchImpl: typeof fetch,
  label: string,
): Promise<Response> {
  let response: Response;
  try {
    response = await fetchImpl(url, {
      redirect: "follow",
      signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT_MS),
    });
  } catch (error) {
    throw new Error(
      `${label} could not be downloaded from ${url}: ${asMessage(error)}.`,
    );
  }

  if (!response.ok) {
    const status = `${response.status}${
      response.statusText ? ` ${response.statusText}` : ""
    }`;
    throw new Error(`${label} is unavailable (HTTP ${status}) at ${url}.`);
  }
  return response;
}

async function isFile(candidate: string): Promise<boolean> {
  try {
    return (await fs.stat(candidate)).isFile();
  } catch {
    return false;
  }
}

async function removeFile(candidate: string): Promise<void> {
  try {
    await fs.rm(candidate, { force: true });
  } catch {
    // Best-effort cleanup; the original error is more useful.
  }
}

function failed(error: unknown): DownloadResult {
  return { ok: false, error: asMessage(error) };
}

function asMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
