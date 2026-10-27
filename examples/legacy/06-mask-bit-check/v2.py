"""Scenario 06: Arbitrary-Mask Bit Checking — Modern eudkit (v2).

Ideal Intent:
Check arbitrary bitmask flags in game state memory (`0x58A364 + 0x200`):
1. Test bit 0 (`0x00000001` - "Shield Ready") -> drive `lamp_shield` (1 or 0)
2. Test bit 5 (`0x00000020` - "Stealth Active") -> drive `lamp_stealth` (1 or 0)
3. Test bit 10 (`0x00000400` - "Hero Aura") -> drive `lamp_aura` (1 or 0)
4. Combined mask check: test if ANY of the three ability bits are active -> drive `lamp_any`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

FLAGS_EPD = EPD(0x58A364 + 0x200)

MASK_SHIELD = 0x00000001
MASK_STEALTH = 0x00000020
MASK_AURA = 0x00000400


def build(session: eudkit.Session):
    lamp_shield = session.var(0, name="lamp_shield")
    lamp_stealth = session.var(0, name="lamp_stealth")
    lamp_aura = session.var(0, name="lamp_aura")
    lamp_any = session.var(0, name="lamp_any")
    flags = session.var(0, name="flags")

    @session.fn
    def game_loop():
        # Materialize flags once with optimized memory reader
        flags = dwread_epd(FLAGS_EPD)

        # Bitwise mask conditions using native & operator
        if flags & MASK_SHIELD != 0:
            lamp_shield = 1
        else:
            lamp_shield = 0

        if flags & MASK_STEALTH != 0:
            lamp_stealth = 1
        else:
            lamp_stealth = 0

        if flags & MASK_AURA != 0:
            lamp_aura = 1
        else:
            lamp_aura = 0

        # Composite mask testing
        if flags & (MASK_SHIELD | MASK_STEALTH | MASK_AURA) != 0:
            lamp_any = 1
        else:
            lamp_any = 0

    return {
        "lamp_shield": lamp_shield,
        "lamp_stealth": lamp_stealth,
        "lamp_aura": lamp_aura,
        "lamp_any": lamp_any,
        "flags": flags,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 06 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
