# 40 - upgrade progress detection (modern eudkit)
#
# Expresses ideal intent:
# 1. Direct byte reads on upgrade table via `bread(...)`.
# 2. Inlined state-transition branches with compile-time constant score payouts.
# 3. Clean local session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

UPG_LEVEL_BASE = 0x58D088
UPGRADES_PER_PLAYER = 61
INFANTRY_ARMOR = 0
INFANTRY_WEAPONS = 7


def build(session: eudkit.Session):
    prev_armor = [session.var(0) for _ in range(2)]
    prev_weapons = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal prev_weapons
        for player in range(2):
            level = eudkit.bread(UPG_LEVEL_BASE + player * UPGRADES_PER_PLAYER + INFANTRY_ARMOR)
            if level > prev_armor[player]:
                if level == 1:
                    eudkit.DisplayText("Infantry Armor Lv1 complete!")
                    eudkit.SetScore(player, eudkit.Add, 50, eudkit.Custom)
                elif level == 2:
                    eudkit.DisplayText("Infantry Armor Lv2 complete!")
                    eudkit.SetScore(player, eudkit.Add, 100, eudkit.Custom)
                elif level == 3:
                    eudkit.DisplayText("Infantry Armor Lv3 complete!")
                    eudkit.SetResources(player, eudkit.Add, 500, eudkit.Ore)
                    eudkit.SetScore(player, eudkit.Add, 150, eudkit.Custom)
                prev_armor[player] = level

        weapons = eudkit.bread(UPG_LEVEL_BASE + 0 * UPGRADES_PER_PLAYER + INFANTRY_WEAPONS)
        if weapons > prev_weapons:
            if weapons == 1:
                eudkit.DisplayText("Infantry Weapons Lv1 complete!")
                eudkit.SetScore(0, eudkit.Add, 40, eudkit.Custom)
            elif weapons == 2:
                eudkit.DisplayText("Infantry Weapons Lv2 complete!")
                eudkit.SetScore(0, eudkit.Add, 80, eudkit.Custom)
            elif weapons == 3:
                eudkit.DisplayText("Infantry Weapons Lv3 complete!")
                eudkit.SetScore(0, eudkit.Add, 120, eudkit.Custom)
            prev_weapons = weapons

    return {"prev_armor": prev_armor, "prev_weapons": prev_weapons}


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 40 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
