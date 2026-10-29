#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const localPackageJson = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
);
const vsix =
  process.argv[2] ?? `dist/eudkit-vscode-${localPackageJson.version}.vsix`;

if (!existsSync(vsix)) {
  console.error(`VSIX not found: ${vsix}`);
  process.exit(1);
}

let entries;
try {
  entries = execFileSync("unzip", ["-Z1", vsix], { encoding: "utf8" })
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .filter(Boolean);
} catch (error) {
  console.error(
    `Could not inspect ${vsix} with unzip. Install unzip or run this check on Linux/macOS: ${error.message}`,
  );
  process.exit(1);
}

const requiredGroups = [
  ["extension/package.json"],
  ["extension/README.md", "extension/readme.md"],
  ["extension/LICENSE", "extension/LICENSE.txt", "extension/LICENSE.md"],
  ["extension/TESTING.md"],
  ["extension/dist/extension.js"],
];
const missing = requiredGroups
  .filter((group) => !group.some((entry) => entries.includes(entry)))
  .map((group) => group[0]);
const forbidden = entries.filter(
  (entry) =>
    entry.includes("node_modules/") ||
    entry === "extension/src" ||
    entry.startsWith("extension/src/") ||
    entry.startsWith("extension/test/") ||
    entry.startsWith("extension/scripts/") ||
    entry.startsWith("extension/.vscode/"),
);

if (missing.length > 0) {
  console.error(`VSIX is missing required files:\n${missing.join("\n")}`);
  process.exit(1);
}
if (forbidden.length > 0) {
  console.error(
    `VSIX contains development-only files:\n${forbidden.join("\n")}`,
  );
  process.exit(1);
}

const packagedPackageJson = JSON.parse(
  execFileSync("unzip", ["-p", vsix, "extension/package.json"], {
    encoding: "utf8",
  }),
);
if (packagedPackageJson.version !== localPackageJson.version) {
  console.error(
    `VSIX package.json has version ${packagedPackageJson.version}, but ` +
      `editors/vscode/package.json has ${localPackageJson.version}`,
  );
  process.exit(1);
}
if (packagedPackageJson.repository?.url !== "https://github.com/Tb1281/eudplib") {
  console.error(
    `VSIX package.json has the wrong repository URL: ${packagedPackageJson.repository?.url}`,
  );
  process.exit(1);
}
if (packagedPackageJson.license !== "MIT") {
  console.error(
    `VSIX package.json has the wrong license: ${packagedPackageJson.license}`,
  );
  process.exit(1);
}

console.log(
  `Verified ${vsix}: ${entries.length} entries, README/LICENSE/TESTING/bundle present, ` +
    `version ${localPackageJson.version}, repository ${packagedPackageJson.repository.url}.`,
);
