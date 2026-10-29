#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const { version } = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
);
const vsce = join(
  "node_modules",
  ".bin",
  process.platform === "win32" ? "vsce.cmd" : "vsce",
);
const output = join("dist", `eudkit-vscode-${version}.vsix`);

execFileSync(vsce, ["package", "--no-dependencies", "--out", output], {
  stdio: "inherit",
  shell: process.platform === "win32",
});
console.log(`Packaged ${output}`);
