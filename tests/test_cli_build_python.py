from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from eudkit import (
    AtLeast,
    Deaths,
    Ore,
    Player1,
    Session,
    SetDeaths,
    SetResources,
    SetTo,
    session,
)
from eudkit.runner import run_build

BASEMAP = Path("tests/outputmap/basemap/basemap_strx.scx").resolve()


class TestCliBuildPython(unittest.TestCase):
    def test_ambient_session_outside_context_raises_runtime_error(self) -> None:
        with self.assertRaises(RuntimeError) as ctx:
            _ = session.var(0)
        self.assertIn("No active eudkit compilation session found", str(ctx.exception))

    def test_ambient_session_inside_context_manager(self) -> None:
        with Session(BASEMAP) as s:
            v1 = session.var(10, name="v1")
            v2 = session.var(20, name="v2")
            self.assertEqual(v1.id, 0)
            self.assertEqual(v2.id, 1)
            self.assertIn("v1", s._vars_by_name)
            self.assertIn("v2", s._vars_by_name)

    def test_python_runner_module_compiles_ambient_script(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            script_path = Path(tmpdir) / "logic.py"
            out_scx = Path(tmpdir) / "output.scx"

            script_path.write_text(
                """from eudkit import session, Deaths, SetDeaths, Player1, AtLeast, SetTo

counter = session.var(0, name="counter")

@session.fn
def game_loop():
    if Deaths(Player1, AtLeast, 1, "Terran Marine"):
        counter += 1
        SetDeaths(Player1, SetTo, 0, "Terran Marine")
"""
            )

            report = run_build(
                input_map=BASEMAP,
                output_map=out_scx,
                entry=script_path,
            )
            self.assertIsNotNone(report)
            self.assertTrue(out_scx.exists())
            self.assertGreater(report.trigger_count, 0)

    def test_python_cli_adhoc_mode(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            script_path = Path(tmpdir) / "adhoc.py"
            out_scx = Path(tmpdir) / "adhoc_out.scx"

            script_path.write_text(
                """from eudkit import session

score = session.var(100, name="score")

@session.fn
def game_loop():
    score += 5
"""
            )

            res = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "eudkit.cli",
                    "build",
                    "--input",
                    str(BASEMAP),
                    "--output",
                    str(out_scx),
                    "--entry",
                    str(script_path),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(res.returncode, 0, f"CLI build failed: {res.stderr}")
            self.assertIn("Built", res.stdout)
            self.assertTrue(out_scx.exists())

    def test_python_cli_project_mode_and_override(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cwd = Path(tmpdir)
            script_path = cwd / "entry.py"
            out1 = cwd / "project_out1.scx"
            out2 = cwd / "project_out2.scx"
            cfg_path = cwd / "eudkit.toml"

            script_path.write_text(
                """from eudkit import session

val = session.var(42, name="val")

@session.fn
def game_loop():
    val += 1
"""
            )

            cfg_path.write_text(
                f"""[project]
name = "proj_test"

[build]
entry = "entry.py"
input_map = "{BASEMAP}"
output_map = "project_out1.scx"
"""
            )

            env = {**os.environ, "PYTHONPATH": str(Path(__file__).resolve().parent.parent / "src")}
            # 1. Zero-arg build inside project directory
            res1 = subprocess.run(
                [sys.executable, "-m", "eudkit.cli", "build"],
                cwd=cwd,
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertEqual(res1.returncode, 0, f"Zero-arg build failed: {res1.stderr}")
            self.assertTrue(out1.exists())

            # 2. CLI flag override (--output)
            res2 = subprocess.run(
                [sys.executable, "-m", "eudkit.cli", "build", "--output", str(out2)],
                cwd=cwd,
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertEqual(res2.returncode, 0, f"Override build failed: {res2.stderr}")
            self.assertTrue(out2.exists())

    def test_python_cli_missing_arguments_errors_cleanly(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            env = {**os.environ, "PYTHONPATH": str(Path(__file__).resolve().parent.parent / "src")}
            res = subprocess.run(
                [sys.executable, "-m", "eudkit.cli", "build"],
                cwd=tmpdir,
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(res.returncode, 0)
            self.assertIn("usage: eudkit build", res.stderr)


    def test_python_session_emits_versioned_hir_plan(self) -> None:
        import json

        with Session(BASEMAP) as sess:
            sess.var(7, name="score")
            sess.var(0, name="flag")

        plan = json.loads(sess.dump_plan())
        self.assertEqual(plan["format"], "eudkit-hir-plan")
        self.assertEqual(plan["schema_version"], 1)
        self.assertEqual(plan["module"]["name"], "eudkit_module")
        self.assertEqual(
            [v["name"] for v in plan["module"]["vars"]], ["score", "flag"]
        )
        self.assertEqual(len(plan["map"]["fingerprint"]), 16)
        self.assertEqual(plan["map"]["name"], BASEMAP.name)

    def test_python_runner_writes_plan_without_compiling(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            script_path = Path(tmpdir) / "logic.py"
            plan_path = Path(tmpdir) / "plan.eudhir"

            script_path.write_text(
                """from eudkit import session

counter = session.var(0, name="counter")

@session.fn
def game_loop():
    counter += 1
"""
            )

            report = run_build(
                input_map=BASEMAP,
                entry=script_path,
                plan=plan_path,
            )
            self.assertIsNone(report)
            self.assertTrue(plan_path.exists())
            text = plan_path.read_text()
            self.assertIn('"format": "eudkit-hir-plan"', text)
            self.assertIn('"name": "counter"', text)


if __name__ == "__main__":
    unittest.main()
