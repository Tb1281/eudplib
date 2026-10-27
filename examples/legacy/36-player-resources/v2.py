# 36 - player resources: periodic bonus / tax (modern eudkit)
#
# Expresses ideal intent:
# 1. Declarative periodic clock via `with every(84):` instead of manual frame counting.
# 2. Native `Accumulate` stock trigger conditions replace 16 expensive memory reads
#    and EUDFunc trampoline overhead.
# 3. Clean local session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

INTERVAL = 84          # ~5 s at Fastest (16.7 trigger loops / s)
LOW_MINERAL = 499      # <= 499 qualifies for relief
BONUS = 150
HIGH_GAS = 301         # >= 301 qualifies for tax
TAX = 50


def build(session: eudkit.Session):
    paid_count = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        with eudkit.every(INTERVAL):
            paid_count = 0
            for player in range(8):
                if eudkit.Accumulate(player, eudkit.AtMost, LOW_MINERAL, eudkit.Ore):
                    eudkit.SetResources(player, eudkit.Add, BONUS, eudkit.Ore)
                    paid_count += 1
                if eudkit.Accumulate(player, eudkit.AtLeast, HIGH_GAS, eudkit.Gas):
                    eudkit.SetResources(player, eudkit.Subtract, TAX, eudkit.Gas)
            if paid_count >= 1:
                eudkit.DisplayText("Relief payments issued.")

    return {"paid_count": paid_count}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 36 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
