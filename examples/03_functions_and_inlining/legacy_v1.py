"""Example 03: Functions & Function Pointers — Legacy eudplib (v1).

Demonstrates:
- @ep.EUDFunc function decorator
- ep.EUDReturn statement
- Calling convention: parameters copied into argument registers, return value copied back
- ep.EUDFuncPtr dynamic dispatch table
- High trigger overhead for small helper functions (no compile-time inlining)
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    # Small helper function: in v1, creates 4+ triggers every time it is called!
    @ep.EUDFunc
    def v1_add3(x, y, z):
        ep.EUDReturn(x + y + z)

    # Worker function for dynamic function pointer
    @ep.EUDFunc
    def v1_worker(x):
        ep.EUDReturn(x + 7)

    @ep.EUDFunc
    def v1_mul(x):
        ep.EUDReturn(x * 3)

    # Dynamic function pointer table
    v1_fptr = ep.EUDFuncPtr(1, 1)(v1_worker)

    def v1_main():
        r1 = ep.EUDVariable(0)
        r2 = ep.EUDVariable(0)
        val = ep.EUDVariable(0)

        # Call helper function (double copy overhead)
        val << v1_add3(5, 15, 10)

        # Call via function pointer
        r1 << v1_fptr(val)

        # Reassign function pointer target
        v1_fptr << v1_mul
        r2 << v1_fptr(val)

    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_main)
    print(f"Legacy v1 functions compiled to: {out_path}")


if __name__ == "__main__":
    main()
