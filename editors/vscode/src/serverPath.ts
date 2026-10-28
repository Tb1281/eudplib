import * as fs from "node:fs";
import * as path from "node:path";

/** Where a resolved `eudkit-lsp` executable came from. */
export type ServerSource = "setting" | "environment" | "path" | "bundled";

export interface SearchAttempt {
  readonly source: ServerSource;
  readonly candidate: string;
  readonly reason?: string;
}

export type ServerResolution =
  | { readonly found: true; readonly path: string; readonly source: ServerSource }
  | { readonly found: false; readonly searched: readonly SearchAttempt[] };

export interface ResolveServerOptions {
  /** Value of the `eudkit.lsp.path` setting. */
  readonly configuredPath?: string;
  /** Value of the `EUDKIT_LSP_PATH` environment variable. */
  readonly environmentPath?: string;
  /** First workspace folder, used to resolve relative paths. */
  readonly workspaceRoot?: string;
  /** Extension installation directory, containing the optional `bin/` directory. */
  readonly extensionRoot: string;
  readonly platform?: NodeJS.Platform;
  readonly arch?: string;
  /** Value of `PATH`. */
  readonly pathValue?: string;
  /** Value of `PATHEXT` on Windows. */
  readonly pathExtensions?: string;
  /** Injectable predicate for unit tests; defaults to a filesystem check. */
  readonly isRunnable?: (candidate: string) => boolean;
}

const SERVER_BASENAME = "eudkit-lsp";
const DEFAULT_WINDOWS_EXTENSIONS = [".EXE", ".CMD", ".BAT"];

/**
 * Resolve the language server using the documented order:
 * setting, environment variable, `PATH`, then the bundled binary.
 *
 * Explicit paths are resolved relative to `workspaceRoot`; if no workspace
 * folder is open, relative paths are rejected with a reason.
 */
export function resolveServer(options: ResolveServerOptions): ServerResolution {
  const platform = options.platform ?? process.platform;
  const arch = options.arch ?? process.arch;
  const isRunnable =
    options.isRunnable ??
    ((candidate: string) => isRunnableFile(candidate, platform));
  const searched: SearchAttempt[] = [];

  const configured = options.configuredPath?.trim();
  if (configured) {
    const resolved = resolvePathValue(configured, options.workspaceRoot);
    if (resolved.path && isRunnable(resolved.path)) {
      return { found: true, path: resolved.path, source: "setting" };
    }
    searched.push({
      source: "setting",
      candidate: resolved.path ?? configured,
      reason: resolved.reason ?? "not found or not executable",
    });
  }

  const environment = options.environmentPath?.trim();
  if (environment) {
    const resolved = resolvePathValue(environment, options.workspaceRoot);
    if (resolved.path && isRunnable(resolved.path)) {
      return { found: true, path: resolved.path, source: "environment" };
    }
    searched.push({
      source: "environment",
      candidate: resolved.path ?? environment,
      reason: resolved.reason ?? "not found or not executable",
    });
  }

  for (const candidate of pathCandidates(
    SERVER_BASENAME,
    platform,
    options.pathValue ?? "",
    options.pathExtensions,
  )) {
    if (isRunnable(candidate)) {
      return { found: true, path: candidate, source: "path" };
    }
    searched.push({
      source: "path",
      candidate,
      reason: "not found or not executable",
    });
  }

  const bundled = bundledServerPath(options.extensionRoot, platform, arch);
  if (isRunnable(bundled)) {
    return { found: true, path: bundled, source: "bundled" };
  }
  searched.push({
    source: "bundled",
    candidate: bundled,
    reason: "not found or not executable",
  });

  return { found: false, searched };
}

/** Path of the optional binary shipped under `bin/<platform>-<arch>/`. */
export function bundledServerPath(
  extensionRoot: string,
  platform: NodeJS.Platform,
  arch: string,
): string {
  const executable =
    platform === "win32" ? `${SERVER_BASENAME}.exe` : SERVER_BASENAME;
  return path.join(extensionRoot, "bin", `${platform}-${arch}`, executable);
}

/** Candidate executable paths from a `PATH`-style value. */
export function pathCandidates(
  basename: string,
  platform: NodeJS.Platform,
  pathValue: string,
  pathExtensions?: string,
): string[] {
  const extensions =
    platform === "win32" ? windowsExtensions(pathExtensions) : [""];
  const candidates: string[] = [];

  for (const rawDirectory of pathValue.split(path.delimiter)) {
    const directory = rawDirectory.trim().replace(/^"(.*)"$/, "$1");
    if (!directory) {
      continue;
    }
    for (const extension of extensions) {
      candidates.push(path.resolve(directory, basename + extension));
    }
  }

  return candidates;
}

function windowsExtensions(pathExtensions?: string): string[] {
  const configured = pathExtensions
    ?.split(";")
    .map((extension) => extension.trim())
    .filter(Boolean);

  const extensions =
    configured && configured.length > 0
      ? configured
      : DEFAULT_WINDOWS_EXTENSIONS;

  return [
    ...new Set(
      extensions.map((extension) =>
        extension.startsWith(".") ? extension : `.${extension}`,
      ),
    ),
  ];
}

function resolvePathValue(
  value: string,
  workspaceRoot?: string,
): { path?: string; reason?: string } {
  if (path.isAbsolute(value)) {
    return { path: path.normalize(value) };
  }
  if (!workspaceRoot) {
    return {
      reason: "relative path requires an open workspace folder",
    };
  }
  return { path: path.resolve(workspaceRoot, value) };
}

function isRunnableFile(candidate: string, platform: NodeJS.Platform): boolean {
  try {
    if (!fs.statSync(candidate).isFile()) {
      return false;
    }
    if (platform !== "win32") {
      fs.accessSync(candidate, fs.constants.X_OK);
    }
    return true;
  } catch {
    return false;
  }
}
