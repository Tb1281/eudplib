"""Example 01: Variables & Arithmetic — Modern eudkit (v2).

Demonstrates:
- Clean ambient Compilation Session (from eudkit import session)
- Unified variable declaration (automatic storage lowering: 4B memory cell vs 72B VProc)
- Natural Python arithmetic expressions (+, -, *, //, &, |, ^)
- Direct action emission without DoActions boilerplate
- Standalone compile via `eudkit build --input ... --output ... --entry eudkit_v2.py`
"""

from eudkit import Player1, SetDeaths, SetTo, session

# 1. Declare variables at module level
# The Rust compiler analyzes variable usage:
# Variables that are only read or assigned constants are lowered to 4-byte ByteBuf cells (0 VProcs).
# Variables requiring dynamic assignment use scratch-reused VProc slots.
a = session.var(10, name="a")
b = session.var(20, name="b")
c = session.var(5, name="c")
d = session.var(0, name="d")


# 2. Define game loop routine using standard Python syntax
@session.fn
def game_loop():
    # Natural compound expressions are compiled into optimized trigger chains
    d = (a + b) - c + (b & 0xFF)

    # Direct, typed action calls without DoActions boilerplate
    SetDeaths(Player1, SetTo, d, "Terran Marine")
