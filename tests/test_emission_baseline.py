"""Byte-for-byte emission baseline captured before the EmissionPlan refactor.

The fixture pins whole-map SHA-256 hashes for representative Python and
TypeScript examples. It exists so that centralizing object layout in
``EmissionPlan`` cannot silently move a trigger, action, byte buffer, or VProc
slot. The expected hashes were produced by the emitter before the refactor.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from eudkit.runner import run_build

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURE_PATH = REPO_ROOT / "tests" / "fixtures" / "emission_baseline" / "baselines.json"
EUDKIT_BIN = REPO_ROOT / "src" / "rust" / "target" / "debug" / "eudkit"

with FIXTURE_PATH.open("r", encoding="utf-8") as fixture_file:
    BASELINES = json.load(fixture_file)

BASEMAP = REPO_ROOT / BASELINES["basemap"]


class TestEmissionBaseline(unittest.TestCase):
    def test_python_frontend_matches_the_pinned_map_hash(self) -> None:
        for case in BASELINES["entries"]:
            if case["frontend"] != "python":
                continue
            with self.subTest(entry=case["entry"]):
                with tempfile.TemporaryDirectory() as tmpdir:
                    output = Path(tmpdir) / "out.scx"
                    report = run_build(
                        input_map=BASEMAP,
                        output_map=output,
                        entry=REPO_ROOT / case["entry"],
                    )
                    self.assertEqual(report.trigger_count, case["trigger_count"])
                    self.assertEqual(report.payload_size, case["payload_size"])
                    self.assertEqual(
                        hashlib.sha256(output.read_bytes()).hexdigest(),
                        case["sha256"],
                        f"{case['entry']} no longer emits byte-identical output",
                    )

    def test_typescript_frontend_matches_the_pinned_map_hash(self) -> None:
        if not EUDKIT_BIN.exists():
            self.skipTest(f"eudkit binary not found at {EUDKIT_BIN}")

        for case in BASELINES["entries"]:
            if case["frontend"] != "typescript":
                continue
            with self.subTest(entry=case["entry"]):
                with tempfile.TemporaryDirectory() as tmpdir:
                    output = Path(tmpdir) / "out.scx"
                    result = subprocess.run(
                        [
                            str(EUDKIT_BIN),
                            "build",
                            "--entry",
                            str(REPO_ROOT / case["entry"]),
                            "--input",
                            str(BASEMAP),
                            "--output",
                            str(output),
                        ],
                        capture_output=True,
                        text=True,
                    )
                    self.assertEqual(
                        result.returncode,
                        0,
                        f"TypeScript build failed for {case['entry']}:\n"
                        f"{result.stderr or result.stdout}",
                    )
                    self.assertEqual(
                        hashlib.sha256(output.read_bytes()).hexdigest(),
                        case["sha256"],
                        f"{case['entry']} no longer emits byte-identical output",
                    )


if __name__ == "__main__":
    unittest.main()
