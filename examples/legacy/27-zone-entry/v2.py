"""Scenario 27: Zone Entry and Dwell Detection — Modern eudkit (v2).

Ideal Intent:
Watch Player 1's Marines entering zone at location 2:
1. Each entry (transition from outside to inside) awards a 25-ore bonus
   (first 10 entries only), pings the zone on the minimap, and increments entries.
2. Each exit (transition from inside to outside) increments exits and clears inside flag.
3. Every loop spent inside accumulates into `loops_inside`.
Modern v2 uses native Python control flow, eliminating manual EUDIf/EUDEndIf boilerplates.
"""

from pathlib import Path
import eudkit
from eudkit import (
    Add,
    AtLeast,
    AtMost,
    Bring,
    MinimapPing,
    Ore,
    Player1,
    SetResources,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

LOC_ZONE = 2
ENTRY_BONUS = 25
PAID_ENTRIES = 10


def build(session: eudkit.Session):
    inside = session.var(0, name="inside")
    entries = session.var(0, name="entries")
    exits = session.var(0, name="exits")
    paid_entries = session.var(0, name="paid_entries")
    bonus_ore_total = session.var(0, name="bonus_ore_total")
    loops_inside = session.var(0, name="loops_inside")

    @session.fn
    def game_loop():
        # Rising edge: outside -> inside
        if Bring(Player1, AtLeast, 1, "Terran Marine", LOC_ZONE):
            if inside == 0:
                entries += 1
                if paid_entries < PAID_ENTRIES:
                    paid_entries += 1
                    bonus_ore_total += ENTRY_BONUS
                    SetResources(Player1, Add, ENTRY_BONUS, Ore)
                MinimapPing(LOC_ZONE)
                inside = 1

        # Falling edge: inside -> outside
        if Bring(Player1, AtMost, 0, "Terran Marine", LOC_ZONE):
            if inside == 1:
                exits += 1
                inside = 0

        # Dwell tracking
        if inside == 1:
            loops_inside += 1

    return {
        "inside": inside,
        "entries": entries,
        "exits": exits,
        "paid_entries": paid_entries,
        "bonus_ore_total": bonus_ore_total,
        "loops_inside": loops_inside,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 27 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
