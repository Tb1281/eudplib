"""Legacy eudplib (v1) deprecation shim.

eudplib has been superseded by eudkit (v2).
See docs/migration-v1-to-v2.md for the complete migration guide.
"""

from __future__ import annotations

import warnings

__version__ = "0.81.0"

warnings.warn(
    "eudplib (v1) is deprecated and has been retired in favor of 'eudkit' (v2). "
    "Please update your imports from 'eudplib' to 'eudkit' for native typed IR compilation, "
    "zero-overhead function inlining, -87% payload reduction, and modern TypeScript support. "
    "See docs/migration-v1-to-v2.md for migration guidance and the 'eudkit migrate' CLI.",
    FutureWarning,
    stacklevel=2,
)

# Re-export modern eudkit symbols so developers can transition seamlessly
import eudkit
from eudkit import *  # noqa: F401, F403


def eudplibVersion() -> str:  # noqa: N802
    """Legacy helper returning current version."""
    return __version__

