"""Parity between the Rust and Python build CLIs and the Python runner report.

Both CLIs resolve `eudkit.toml` through `BuildRequest`, and the Python runner
hands its real `CompilationReport` back to the Rust CLI as typed JSON instead of
the Rust CLI reconstructing a zeroed report from the output map.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from eudkit import _rust
from eudkit.runner import run_build

REPO_ROOT = Path(__file__).resolve().parent.parent
BASEMAP = (REPO_ROOT / "tests/outputmap/basemap/basemap_strx.scx").resolve()
EUDKIT_BIN = REPO_ROOT / "src/rust/target/debug/eudkit"
ENTRY = """from eudkit import session

counter = session.var(0, name="counter")

@session.fn
def game_loop():
    counter += 1
"""


def subprocess_env() -> dict[str, str]:
    return {**os.environ, "PYTHONPATH": str(REPO_ROOT / "src")}


class TestBuildRequestParity(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not EUDKIT_BIN.exists():
            raise unittest.SkipTest(f"eudkit binary not found at {EUDKIT_BIN}")

    def test_python_resolver_shares_section_aliases_and_flag_precedence(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cwd = Path(tmpdir)
            config = cwd / "eudkit.toml"
            config.write_text(
                f"""[project]
name = "resolved"
entry = "entry.py"
input_map = "project_in.scx"
dts = "types.d.ts"
obscurate = false
obscurate_seed = 11
preserve_classic_triggers = false

[build]
input_map = "build_in.scx"
output_map = "build_out.scx"
obscurate_seed = 22
unknown_key = "ignored"
""",
                encoding="utf-8",
            )

            resolved = _rust.resolve_build_request(
                config=str(config),
                input_map="flag_in.scx",
                obscurate=True,
                obscurate_seed=33,
            )

            self.assertEqual(resolved["name"], "resolved")
            self.assertEqual(resolved["entry"], "entry.py")
            self.assertEqual(resolved["input_map"], "flag_in.scx")
            self.assertEqual(resolved["output_map"], str(cwd / "build_out.scx"))
            self.assertEqual(resolved["dts"], str(cwd / "types.d.ts"))
            self.assertTrue(resolved["obscurate"])
            self.assertEqual(resolved["obscurate_seed"], 33)
            self.assertFalse(resolved["preserve_classic_triggers"])
            self.assertEqual(resolved["config_path"], str(config))

    def test_python_and_rust_cli_build_the_same_project_byte_for_byte(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cwd = Path(tmpdir)
            (cwd / "entry.py").write_text(ENTRY, encoding="utf-8")
            (cwd / "eudkit.toml").write_text(
                f"""[project]
name = "parity"
entry = "entry.py"
input_map = "{BASEMAP}"

[build]
output_map = "python_out.scx"
obscurate_seed = 9
preserve_classic_triggers = false
""",
                encoding="utf-8",
            )

            python_build = subprocess.run(
                [sys.executable, "-m", "eudkit.cli", "build"],
                cwd=cwd,
                env=subprocess_env(),
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                python_build.returncode,
                0,
                f"Python CLI build failed:\n{python_build.stderr}",
            )

            rust_build = subprocess.run(
                [str(EUDKIT_BIN), "build", "--output", "rust_out.scx"],
                cwd=cwd,
                env=subprocess_env(),
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                rust_build.returncode,
                0,
                f"Rust CLI build failed:\n{rust_build.stderr}",
            )
            self.assertIn("Built", rust_build.stdout)
            self.assertIn("triggers=", rust_build.stdout)

            python_bytes = (cwd / "python_out.scx").read_bytes()
            rust_bytes = (cwd / "rust_out.scx").read_bytes()
            self.assertEqual(python_bytes, rust_bytes)

    def test_runner_writes_typed_json_report_and_summary_for_the_rust_cli(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cwd = Path(tmpdir)
            entry = cwd / "entry.py"
            entry.write_text(ENTRY, encoding="utf-8")
            output = cwd / "out.scx"
            report_path = cwd / "report.json"

            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "eudkit.runner",
                    "--input",
                    str(BASEMAP),
                    "--output",
                    str(output),
                    "--entry",
                    str(entry),
                    "--report",
                    str(report_path),
                ],
                cwd=cwd,
                env=subprocess_env(),
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                result.returncode,
                0,
                f"Python runner failed:\n{result.stderr}",
            )

            values = json.loads(report_path.read_text(encoding="utf-8"))
            for field in (
                "trigger_count",
                "byte_buf_count",
                "vproc_count",
                "payload_offset",
                "payload_size",
                "payload_entry_addr",
                "prt_offsets",
                "ort_offsets",
            ):
                self.assertIn(field, values)
            self.assertGreater(values["trigger_count"], 0)
            self.assertGreater(values["payload_size"], 0)
            self.assertGreater(values["payload_entry_addr"], 0)
            self.assertGreater(len(values["prt_offsets"]), 0)

            direct = run_build(input_map=BASEMAP, output_map=output, entry=entry)
            self.assertEqual(direct.prt_count, len(direct.prt_offsets))
            self.assertEqual(direct.ort_count, len(direct.ort_offsets))
            self.assertEqual(values, json.loads(direct.to_json()))
            self.assertIn(direct.summary(str(output)), result.stdout)


if __name__ == "__main__":
    unittest.main()
