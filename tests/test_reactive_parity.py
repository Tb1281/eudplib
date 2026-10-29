"""Cross-Frontend parity for the five Reactive Rule primitives.

Phase (a) lives here: both Frontends must emit the same pre-lowering
`HirStmt::Reactive` blocks for the shared fixture. Phase (b) lives in
`src/rust/eudkit-core/tests/reactive_rules_test.rs`, which owns the canonical
guard/timer lowering.

The test drives the existing `eudkit dump-ir` seam rather than adding a second
Rust serialization path.
"""

from __future__ import annotations

import re
import subprocess
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR = REPO_ROOT / "tests" / "fixtures" / "reactive_parity"
PY_FIXTURE = FIXTURE_DIR / "rules.py"
TS_FIXTURE = FIXTURE_DIR / "rules.ts"
EXPECTED_HIR = FIXTURE_DIR / "expected_hir.txt"
TS_SDK_INDEX = REPO_ROOT / "packages" / "ts-sdk" / "src" / "index.ts"
TESTMAP_PATH = REPO_ROOT / "tests" / "unittests" / "testmap.scx"
EUDKIT_BIN = REPO_ROOT / "src" / "rust" / "target" / "debug" / "eudkit"

REACTIVE_HEADER = re.compile(
    r"^(?:once|when|until) \{$|^(?:after|every) \(\d+ loops\) \{$"
)
VAR_ID = re.compile(r"@v\d+")
BLOCK_ID = re.compile(r"\bbb\d+\b")
FUNC_ID = re.compile(r"@fn\d+")
FUNC_PTR_ID = re.compile(r"@fp\d+")


def _normalize_identity(text: str) -> str:
    """Strip compile-session-specific variable, block, and function identifiers."""
    text = VAR_ID.sub("@var", text)
    text = BLOCK_ID.sub("bb", text)
    text = FUNC_ID.sub("@fn", text)
    text = FUNC_PTR_ID.sub("@fp", text)
    return text.rstrip()


def reactive_projection(hir: str) -> list[str]:
    """Extract and normalize the pre-lowering Reactive blocks from a HIR dump."""
    lines = hir.splitlines()
    blocks: list[list[str]] = []
    index = 0
    while index < len(lines):
        if not REACTIVE_HEADER.match(lines[index].strip()):
            index += 1
            continue

        block: list[str] = []
        depth = 0
        while index < len(lines):
            line = lines[index]
            block.append(line)
            depth += line.count("{") - line.count("}")
            index += 1
            if depth == 0:
                break
        blocks.append(block)

    projection: list[str] = []
    for block in blocks:
        indents = [len(line) - len(line.lstrip()) for line in block if line.strip()]
        base_indent = min(indents) if indents else 0
        for line in block:
            if line.strip():
                projection.append(_normalize_identity(line[base_indent:]))
            else:
                projection.append("")
    return projection


class TestReactiveRuleParity(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not EUDKIT_BIN.exists():
            raise unittest.SkipTest(f"eudkit binary not found at {EUDKIT_BIN}")

    def dump_hir(self, entry: Path) -> str:
        result = subprocess.run(
            [
                str(EUDKIT_BIN),
                "dump-ir",
                "--map",
                str(TESTMAP_PATH),
                "--entry",
                str(entry),
                "--hir-only",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"dump-ir failed for {entry}:\n{result.stderr or result.stdout}",
        )
        return result.stdout

    def test_pre_lowering_reactive_blocks_match_across_frontends(self) -> None:
        python_hir = reactive_projection(self.dump_hir(PY_FIXTURE))
        typescript_hir = reactive_projection(self.dump_hir(TS_FIXTURE))
        expected = [
            line
            for line in EXPECTED_HIR.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]

        self.assertEqual(python_hir, typescript_hir)
        self.assertEqual(python_hir, expected)

        headers = [line for line in python_hir if REACTIVE_HEADER.match(line)]
        self.assertEqual(len(headers), 6, "fixture must cover all five rule kinds")
        self.assertEqual(
            {line.split()[0] for line in headers},
            {"once", "when", "until", "after", "every"},
        )

    def test_ts_sdk_reactive_functions_are_compile_time_markers(self) -> None:
        source = TS_SDK_INDEX.read_text(encoding="utf-8")

        for name in ("once", "when", "until", "after", "every"):
            self.assertRegex(
                source,
                rf"export declare function {name}\(",
                f"{name} must be an ambient compile-time declaration",
            )
            self.assertNotRegex(
                source,
                rf"export function {name}\(",
                f"{name} must not ship a runtime implementation",
            )

        self.assertNotIn(
            "Boolean(condition)",
            source,
            "the TS SDK must not re-implement Reactive Rule truth evaluation",
        )


if __name__ == "__main__":
    unittest.main()
