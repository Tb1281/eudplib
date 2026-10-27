"""Scenario 34: Units.dat In-Memory Patch and Verification — Modern eudkit (v2).

Ideal Intent:
At map initialization, patch Terran infantry stats in `Units.dat` table memory:
- Marine (0): 80 HP, 1 Armor
- Ghost (1): 60 HP, 1 Armor
- Firebat (32): 70 HP, 1 Armor
- Medic (34): 75 HP, 1 Armor
HitPoints are stored shifted by 8 bits (`HP * 256`) at `0x662350 + 4 * unit_id`.
Armor is stored as an 8-bit byte at `0x65FEC8 + unit_id`.
After writing, reads back all values via unified ReadTable to verify patches stuck:
- If all match: announces `"Terran infantry buffed: HP up, armor 1."`
- Otherwise: announces `"WARNING: infantry buff did not stick."`
Modern v2 uses:
- Declarative `with once():` lifecycle block.
- Direct memory I/O primitives (`dwwrite`, `bwrite`, `dwread`, `bread`).
- Direct string literals in `DisplayText` backed by STRx deduplication.
"""

from pathlib import Path
import eudkit
from eudkit import (
    DisplayText,
    bread,
    bwrite,
    dwread,
    dwwrite,
    once,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

HP_BASE = 0x662350
ARMOR_BASE = 0x65FEC8

MARINE = 0
GHOST = 1
FIREBAT = 32
MEDIC = 34


def build(session: eudkit.Session):
    verify_failed = session.var(0, name="verify_failed")

    @session.fn
    def init_patch():
        with once():
            verify_failed = 0

            # 1. Write patches for each infantry unit
            dwwrite(HP_BASE + 4 * MARINE, 80 * 256)
            bwrite(ARMOR_BASE + MARINE, 1)

            dwwrite(HP_BASE + 4 * GHOST, 60 * 256)
            bwrite(ARMOR_BASE + GHOST, 1)

            dwwrite(HP_BASE + 4 * FIREBAT, 70 * 256)
            bwrite(ARMOR_BASE + FIREBAT, 1)

            dwwrite(HP_BASE + 4 * MEDIC, 75 * 256)
            bwrite(ARMOR_BASE + MEDIC, 1)

            # 2. Read-back verification
            hp0 = dwread(HP_BASE + 4 * MARINE)
            arm0 = bread(ARMOR_BASE + MARINE)
            if hp0 != 80 * 256:
                verify_failed += 1
            if arm0 != 1:
                verify_failed += 1

            hp1 = dwread(HP_BASE + 4 * GHOST)
            arm1 = bread(ARMOR_BASE + GHOST)
            if hp1 != 60 * 256:
                verify_failed += 1
            if arm1 != 1:
                verify_failed += 1

            hp32 = dwread(HP_BASE + 4 * FIREBAT)
            arm32 = bread(ARMOR_BASE + FIREBAT)
            if hp32 != 70 * 256:
                verify_failed += 1
            if arm32 != 1:
                verify_failed += 1

            hp34 = dwread(HP_BASE + 4 * MEDIC)
            arm34 = bread(ARMOR_BASE + MEDIC)
            if hp34 != 75 * 256:
                verify_failed += 1
            if arm34 != 1:
                verify_failed += 1

            # 3. Announcement
            if verify_failed == 0:
                DisplayText("Terran infantry buffed: HP up, armor 1.")
            else:
                DisplayText("WARNING: infantry buff did not stick.")

    return {
        "verify_failed": verify_failed,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 34 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
