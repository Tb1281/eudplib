"""Parity tests for the shared Frontend vocabulary manifest."""

from __future__ import annotations

import unittest


class TestVocabularyParity(unittest.TestCase):
    def test_python_constants_match_the_rust_manifest(self) -> None:
        import eudkit.types as types
        from eudkit import _rust

        manifest = _rust.vocabulary_constants()
        for name, code in manifest.items():
            self.assertEqual(getattr(types, name), code, name)

    def test_force1_and_switch_spellings_are_stable(self) -> None:
        import eudkit.types as types
        from eudkit import _rust

        self.assertEqual(types.Force1, 18)
        self.assertEqual(types.NonAlliedVictoryPlayers, 26)
        self.assertEqual(types.Set, 2)
        self.assertEqual(types.Clear, 5)
        self.assertEqual(types.Random, 7)
        self.assertEqual(_rust.RawAction.set_switch(0, types.Random).__repr__().count("Random"), 1)

    def test_invalid_switch_action_still_raises_value_error(self) -> None:
        from eudkit import _rust

        with self.assertRaises(ValueError):
            _rust.RawAction.set_switch(0, 8)


if __name__ == "__main__":
    unittest.main()
