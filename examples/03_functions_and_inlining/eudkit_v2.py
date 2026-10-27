"""Example 03: Functions & Inlining — Modern eudkit (v2).

Demonstrates:
- @session.fn(inline=True): Zero-overhead function inlining at HIR level
- @session.fn(inline=False): Single-action direct-return calling convention
- session.func_ptr(n_args, n_rets, init=fn): Dynamic indirect function calls
- Standard Python return statements
"""

from pathlib import Path
import eudkit

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    session = eudkit.Session(BASEMAP_PATH)
    r1 = session.var(0, name="r1")
    r2 = session.var(0, name="r2")
    val = session.var(0, name="val")

    # 1. Inlined small helper function:
    # Compiles with 0 call overhead! Its AST is substituted directly into the caller.
    @session.fn(inline=True)
    def add3(x, y, z):
        return x + y + z

    # 2. Non-inlined function using direct-return calling convention:
    # Writes the return value directly into the caller's target variable in 1 action.
    @session.fn(inline=False)
    def worker(x):
        return x + 7

    @session.fn(inline=False)
    def mul(x):
        return x * 3

    # 3. Dynamic function pointer with indirect call
    fptr = session.func_ptr(1, 1, init=worker, name="fptr")

    @session.fn
    def game_loop():
        # Inlined call folds into direct arithmetic
        val = add3(5, 15, 10)

        # Call via function pointer
        r1 = fptr(val)

        # Reassign target dynamically
        fptr.set(mul)
        r2 = fptr(val)

    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)

    print("=== eudkit (v2) Functions Compilation Report ===")
    print(f"Total Triggers:   {report.trigger_count}")
    print(f"Payload Size:     {report.payload_size} bytes")
    print(f"Output SCX Map:   {out_path}")


if __name__ == "__main__":
    main()
