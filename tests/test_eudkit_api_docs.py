"""Documentation coverage for the public eudkit API surface.

These tests keep editor hover useful: any function or class re-exported from
``eudkit.types`` must carry a docstring, and the ``eud`` namespace helpers used
by modern v2 scripts must be documented as well.
"""

from __future__ import annotations

import ast
import inspect
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
INIT_PATH = REPO_ROOT / "src" / "eudkit" / "__init__.py"

# Functions called out by the LSP symbol table and the issue acceptance criteria.
REQUIRED_DOCUMENTED_APIS = {
    "KillUnit",
    "Deaths",
    "DeathsX",
    "SetDeaths",
    "SetDeathsX",
    "Bring",
    "CreateUnit",
    "KillUnitAt",
    "RemoveUnitAt",
    "MinimapPing",
    "MoveLocation",
    "MoveUnit",
    "GiveUnits",
    "Order",
    "Switch",
    "SetSwitch",
}

REQUIRED_EUD_MEMBERS = ("fn", "func_ptr", "loop_range", "do")


def exported_names() -> list[str]:
    """Read the ``__all__`` list from ``src/eudkit/__init__.py``."""
    tree = ast.parse(INIT_PATH.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id == "__all__"
            for target in node.targets
        ):
            return list(ast.literal_eval(node.value))
    raise AssertionError("src/eudkit/__init__.py does not define __all__")


class TestPublicApiDocumentation(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        import eudkit

        cls.eudkit = eudkit
        cls.names = exported_names()

    def test_exported_types_functions_and_classes_have_docstrings(self) -> None:
        covered: list[str] = []
        missing: list[str] = []

        for name in self.names:
            obj = getattr(self.eudkit, name)
            is_public_api = inspect.isfunction(obj) or inspect.isclass(obj)
            if not is_public_api or getattr(obj, "__module__", None) != "eudkit.types":
                continue
            covered.append(name)
            if not inspect.getdoc(obj):
                missing.append(name)

        self.assertGreaterEqual(
            len(covered), 60, "expected to find the eudkit.types public API surface"
        )
        self.assertTrue(
            REQUIRED_DOCUMENTED_APIS.issubset(covered),
            f"missing required APIs from exports: {sorted(REQUIRED_DOCUMENTED_APIS - set(covered))}",
        )
        self.assertEqual(
            missing,
            [],
            "public eudkit.types APIs without a docstring: " + ", ".join(sorted(missing)),
        )

    def test_eud_namespace_helpers_have_docstrings(self) -> None:
        missing = [
            member
            for member in REQUIRED_EUD_MEMBERS
            if not inspect.getdoc(getattr(self.eudkit.eud, member))
        ]
        self.assertEqual(
            missing,
            [],
            "eud namespace helpers without a docstring: " + ", ".join(missing),
        )

    def test_kill_unit_documents_wrapper_and_raw_trigger_order(self) -> None:
        doc = inspect.getdoc(self.eudkit.KillUnit) or ""
        self.assertIn("(unit, player)", doc)
        self.assertIn("(player, unit, amount)", doc)


if __name__ == "__main__":
    unittest.main()
