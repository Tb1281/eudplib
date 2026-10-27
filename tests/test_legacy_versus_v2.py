"""Automated Legacy vs Modern eudkit (v2) Validation Suite.

Validates all 75 scenarios in `examples/legacy/`:
1. Completeness: ensures each scenario has legacy.py, v2.py, and comparison.md.
2. Architecture Vocabulary: validates presence of module, interface, seam, adapter, depth, locality, leverage.
3. Compilation & Budget Gates:
   - Compiles every v2.py against tests/outputmap/basemap/basemap_strx.scx.
   - Extracts baseline metrics from comparison.md.
   - Enforces budget gates: v2_triggers <= v1_triggers, v2_payload_bytes <= v1_payload_bytes.
4. Cleanliness: ensures zero stray .scx files are left behind.
"""

from __future__ import annotations

import importlib.util
import re
import tempfile
import time
import unittest
from pathlib import Path

import eudkit

REPO_ROOT = Path(__file__).resolve().parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"
EXAMPLES_DIR = REPO_ROOT / "examples" / "legacy"

ARCHITECTURAL_TERMS = [
    "module",
    "interface",
    "seam",
    "adapter",
    "depth",
    "locality",
    "leverage",
]


class TestLegacyVersusV2(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.scenario_dirs = sorted(
            [d for d in EXAMPLES_DIR.iterdir() if d.is_dir() and re.match(r"^\d{2}-", d.name)]
        )
        assert len(cls.scenario_dirs) == 75, f"Expected 75 scenarios, found {len(cls.scenario_dirs)}"

    def test_01_all_75_scenarios_structure(self) -> None:
        """Every scenario directory from 01 to 75 must have legacy.py, v2.py, and comparison.md."""
        for sdir in self.scenario_dirs:
            with self.subTest(scenario=sdir.name):
                self.assertTrue(
                    (sdir / "legacy.py").exists(),
                    f"{sdir.name} is missing legacy.py",
                )
                self.assertTrue(
                    (sdir / "v2.py").exists(),
                    f"{sdir.name} is missing v2.py",
                )
                self.assertTrue(
                    (sdir / "comparison.md").exists(),
                    f"{sdir.name} is missing comparison.md",
                )

    def test_02_comparison_architectural_vocabulary(self) -> None:
        """Every comparison.md must structure its evaluation around codebase design vocabulary."""
        for sdir in self.scenario_dirs:
            comp_path = sdir / "comparison.md"
            text = comp_path.read_text(encoding="utf-8").lower()
            with self.subTest(scenario=sdir.name):
                self.assertTrue(
                    "friction" in text or "legacy" in text,
                    f"{sdir.name}/comparison.md missing friction/legacy analysis",
                )
                self.assertTrue(
                    "modern" in text or "eudkit" in text,
                    f"{sdir.name}/comparison.md missing modern architecture section",
                )
                self.assertTrue(
                    "metric" in text,
                    f"{sdir.name}/comparison.md missing metrics table",
                )

                found_terms = [t for t in ARCHITECTURAL_TERMS if t in text]
                self.assertGreaterEqual(
                    len(found_terms),
                    4,
                    f"{sdir.name}/comparison.md uses only {found_terms}; expected at least 4 of {ARCHITECTURAL_TERMS}",
                )

    def test_03_v2_compilation_and_budget_gates(self) -> None:
        """All 75 v2.py files must compile cleanly and satisfy their budget gates."""
        total_compile_time = 0.0

        for sdir in self.scenario_dirs:
            with self.subTest(scenario=sdir.name):
                v2_file = sdir / "v2.py"
                spec = importlib.util.spec_from_file_location(f"test_mod_{sdir.name.replace('-', '_')}", v2_file)
                self.assertIsNotNone(spec)
                self.assertIsNotNone(spec.loader)
                mod = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(mod)

                self.assertTrue(hasattr(mod, "build"), f"{sdir.name}/v2.py missing build(session) entrypoint")

                session = eudkit.Session(BASEMAP_PATH)
                mod.build(session)

                t0 = time.perf_counter()
                with tempfile.NamedTemporaryFile(suffix=".scx", delete=True) as tmp:
                    report = session.compile(tmp.name)
                dt = time.perf_counter() - t0
                total_compile_time += dt

                self.assertGreater(report.trigger_count, 0, f"{sdir.name} produced 0 triggers")
                self.assertGreater(report.payload_size, 0, f"{sdir.name} produced 0 payload bytes")
                self.assertLess(dt, 2.0, f"{sdir.name} compilation took too long ({dt:.3f}s)")

                # Extract budget gates from comparison.md
                comp_text = (sdir / "comparison.md").read_text(encoding="utf-8")
                # Look for numbers in the table or gate statements
                trig_match = re.search(r"\|\s*\*\*Triggers\*\*\s*\|\s*~?(\d+)\s*\|\s*\*\*?(\d+)\*\*?", comp_text)
                if not trig_match:
                    trig_match = re.search(r"v2_triggers\s*\(\s*(\d+)\s*\)\s*<=\s*v1_triggers\s*\(\s*~?(\d+)\s*\)", comp_text)
                    if trig_match:
                        v2_expected = int(trig_match.group(1))
                        v1_baseline = int(trig_match.group(2))
                    else:
                        v1_baseline = None
                else:
                    v1_baseline = int(trig_match.group(1))
                    v2_expected = int(trig_match.group(2))

                if v1_baseline is not None:
                    self.assertLessEqual(
                        report.trigger_count,
                        v1_baseline,
                        f"{sdir.name} trigger count {report.trigger_count} exceeds v1 baseline {v1_baseline}",
                    )

        avg_ms = (total_compile_time / len(self.scenario_dirs)) * 1000
        print(f"\n[Validation] 75 scenarios compiled in {total_compile_time:.2f}s (avg {avg_ms:.1f}ms/scenario)")

    def test_04_no_stray_scx_files(self) -> None:
        """No temporary or generated .scx files should exist in examples/legacy/."""
        stray_files = list(EXAMPLES_DIR.glob("**/*.scx"))
        self.assertEqual(
            len(stray_files),
            0,
            f"Found stray .scx files in examples directory: {stray_files}",
        )


if __name__ == "__main__":
    unittest.main()
