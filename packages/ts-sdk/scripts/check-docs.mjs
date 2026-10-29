#!/usr/bin/env node
/**
 * Checks that every top-level `export function` and `export class` in the
 * TypeScript SDK has a non-empty TSDoc (`/** ... *\/`) block immediately above
 * it. Runs as part of `npm test`.
 */
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const sourcePath = path.resolve(scriptDir, "../src/index.ts");
const source = await readFile(sourcePath, "utf8");
const lines = source.split(/\r?\n/);

const DECLARATION = /^export\s+(function|class)\s+([A-Za-z_$][\w$]*)/;
const MIN_EXPECTED_DECLARATIONS = 60;

/** Returns the TSDoc text immediately above `index`, or null when absent/empty. */
function tsdocAbove(index) {
  let cursor = index - 1;
  while (cursor >= 0 && lines[cursor].trim() === "") {
    cursor--;
  }
  if (cursor < 0 || !lines[cursor].trim().endsWith("*/")) {
    return null;
  }

  const block = [];
  const end = cursor;
  while (cursor >= 0) {
    const trimmed = lines[cursor].trim();
    if (trimmed.startsWith("/**")) {
      block.push(trimmed.slice(3));
      const content = block
        .join("\n")
        .replace(/\*\/\s*$/, "")
        .split("\n")
        .map((line) => line.replace(/^\s*\*?\s?/, "").trim())
        .filter(Boolean)
        .join("\n");
      return content.length > 0 ? content : null;
    }
    if (trimmed.startsWith("/*")) {
      return null;
    }
    block.push(trimmed);
    cursor--;
  }
  // A stray `*/` without a matching `/**` opener is not a TSDoc block.
  return null;
}

const declarations = [];
for (let index = 0; index < lines.length; index++) {
  const match = lines[index].match(DECLARATION);
  if (!match) {
    continue;
  }
  declarations.push({
    kind: match[1],
    name: match[2],
    line: index + 1,
    documented: tsdocAbove(index) !== null,
  });
}

const missing = declarations.filter((declaration) => !declaration.documented);
const relativeSource = path.relative(process.cwd(), sourcePath);

console.log(
  `Checking TSDoc coverage in ${relativeSource}: ` +
    `${declarations.length} exported functions/classes.`
);

if (declarations.length < MIN_EXPECTED_DECLARATIONS) {
  console.error(
    `error: expected at least ${MIN_EXPECTED_DECLARATIONS} exported declarations, found ` +
      `${declarations.length}. The coverage check may be parsing the wrong file.`
  );
  process.exit(1);
}

if (missing.length > 0) {
  console.error("\nMissing TSDoc:");
  for (const declaration of missing) {
    console.error(`  - ${declaration.kind} ${declaration.name} (line ${declaration.line})`);
  }
  console.error(
    `\nerror: ${missing.length} of ${declarations.length} exported functions/classes ` +
      "lack a TSDoc block."
  );
  process.exit(1);
}

console.log(`All ${declarations.length} exported functions/classes are documented.`);
