# 74 - random box shop (modern eudkit)
#
# Expresses ideal intent:
# 1. Purchase gate evaluation with stock `MoveUnit` shop expulsion.
# 2. Inlined LCG probability roll with weighted reward distribution.
# 3. Deduplicated map strings and clean session state encapsulation.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
CIVILIAN = 15
MARINE = 0
BATTLECRUISER = 12
SHOP = 60
SHOP_EXIT = 61
BASE = 30
PRICE = 50
COOLDOWN_LOOPS = 120


def build(session: eudkit.Session):
    cooldown = session.var(0)
    seed = session.var(987654321)
    roll = session.var(0)
    boxes_opened = session.var(0)
    consolation_count = session.var(0)
    mineral_count = session.var(0)
    army_count = session.var(0)
    jackpot_count = session.var(0)
    ore_spent = session.var(0)
    ore_won_back = session.var(0)

    msg_consolation = session.map_string("Random box: 25 ore consolation.")
    msg_mineral = session.map_string("Random box: 100 ore mineral pack!")
    msg_army = session.map_string("Random box: 2 Marines join you!")
    msg_jackpot = session.map_string("Random box: JACKPOT - a Battlecruiser!")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal cooldown, seed, roll, boxes_opened, consolation_count
        nonlocal mineral_count, army_count, jackpot_count, ore_spent, ore_won_back

        if cooldown >= 1:
            cooldown -= 1

        if cooldown == 0 and eudkit.Bring(P1, eudkit.AtLeast, 1, CIVILIAN, SHOP) and eudkit.Accumulate(P1, eudkit.AtLeast, PRICE, eudkit.Ore):
            eudkit.SetResources(P1, eudkit.Subtract, PRICE, eudkit.Ore)
            ore_spent += PRICE
            eudkit.MoveUnit(1, CIVILIAN, P1, SHOP, SHOP_EXIT)
            cooldown = COOLDOWN_LOOPS
            boxes_opened += 1

            seed = seed * 1103515245 + 12345
            roll = (seed >> 16) % 100

            if roll < 50:
                eudkit.SetResources(P1, eudkit.Add, 25, eudkit.Ore)
                ore_won_back += 25
                consolation_count += 1
                msg_consolation.display()
            elif roll < 80:
                eudkit.SetResources(P1, eudkit.Add, 100, eudkit.Ore)
                ore_won_back += 100
                mineral_count += 1
                msg_mineral.display()
            elif roll < 95:
                eudkit.CreateUnit(2, MARINE, BASE, P1)
                army_count += 1
                msg_army.display()
            else:
                eudkit.CreateUnit(1, BATTLECRUISER, BASE, P1)
                jackpot_count += 1
                msg_jackpot.display()

    return {
        "cooldown": cooldown,
        "boxes_opened": boxes_opened,
        "consolation_count": consolation_count,
        "mineral_count": mineral_count,
        "army_count": army_count,
        "jackpot_count": jackpot_count,
        "ore_spent": ore_spent,
        "ore_won_back": ore_won_back,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 74 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
