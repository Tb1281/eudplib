"""Backward compatibility adapter during v1-to-v2 migration.

Delegates native calls to the dedicated `eudkit._rust` module.
"""

from __future__ import annotations

import sys
from eudkit import _rust as _rust_mod

allocator = getattr(_rust_mod, "allocator", None)
epscript = getattr(_rust_mod, "epscript", None)
eudobj = getattr(_rust_mod, "eudobj", None)
eudkit = getattr(_rust_mod, "eudkit", _rust_mod)
mpqapi = getattr(_rust_mod, "mpqapi", None)
register_translator = getattr(_rust_mod, "register_translator", None)

sys.modules["eudplib.bindings._rust"] = _rust_mod
