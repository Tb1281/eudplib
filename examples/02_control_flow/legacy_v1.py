"""Example 02: Control Flow — Legacy eudplib (v1).

Demonstrates:
- Leaky control-flow macros: EUDIf, EUDElseIf, EUDElse, EUDEndIf
- Leaky loop macros: EUDWhile, EUDContinue, EUDBreak, EUDEndWhile
- Comparison syntax: (a >= 30), (a == 15)
- Inability to use native Python if/while statements
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    def v1_control_flow():
        a = ep.EUDVariable(0)
        b = ep.EUDVariable(20)
        c = ep.EUDVariable(0)

        # 1. Multi-branch conditional requiring nested function call syntax
        if ep.EUDIf()(a >= 30):
            c << 100
            ep.DoActions(ep.SetResources(ep.Player1, ep.Add, 500, ep.Ore))
        if ep.EUDElseIf()(a == 15):
            c << 50
        if ep.EUDElse()():
            c << 10
        ep.EUDEndIf()

        # 2. While loop requiring explicit while/continue/break/end markers
        if ep.EUDWhile()(b >= 1):
            b -= 1
            if ep.EUDIf()(b == 5):
                a += 10
                ep.EUDContinue()  # Jumps to loop header trigger
            ep.EUDEndIf()

            if ep.EUDIf()(a >= 200):
                ep.EUDBreak()  # Jumps to loop exit trigger
            ep.EUDEndIf()

            a += 2
        ep.EUDEndWhile()

    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_control_flow)
    print(f"Legacy v1 control flow compiled to: {out_path}")


if __name__ == "__main__":
    main()
