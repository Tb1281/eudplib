"""Scenario 09: Struct-Array Element Field Access — Modern eudkit (v2).

Ideal Intent:
Manage a table of 8 structured unit records (84 dwords per record, matching CUnit stride):
1. Pass 1: Find the first free slot (`hp == 0`), respawn it (`hp = 1000`), record 1-based index,
   and terminate loop early (`break`).
2. Pass 2: Iterate over live units (`hp != 0`) and decrement energy (`energy - 1`) if `energy >= 1`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
    dwwrite_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

STRIDE = 84  # dwords per record (CUnit size: 336 bytes)
DW_HP = 2  # HP field: byte offset +0x08
DW_ENERGY = 48  # Energy field: byte offset +0xC0 (192 bytes)
REC_BUF_EPD = EPD(0x6FE000)


def build(session: eudkit.Session):
    idx = session.var(0, name="idx")
    respawned = session.var(0, name="respawned")
    slot_epd = session.var(0, name="slot_epd")
    hp = session.var(0, name="hp")
    energy = session.var(0, name="energy")

    @session.fn
    def game_loop():
        # Pass 1: Find first free slot and respawn
        idx = 0
        respawned = 0
        while idx < 8:
            slot_epd = REC_BUF_EPD + idx * STRIDE
            hp = dwread_epd(slot_epd + DW_HP)
            if hp == 0:
                dwwrite_epd(slot_epd + DW_HP, 1000)
                respawned = idx + 1
                break
            idx += 1

        # Pass 2: Drain energy of active units
        idx = 0
        while idx < 8:
            slot_epd = REC_BUF_EPD + idx * STRIDE
            hp = dwread_epd(slot_epd + DW_HP)
            energy = dwread_epd(slot_epd + DW_ENERGY)
            if hp != 0:
                if energy >= 1:
                    dwwrite_epd(slot_epd + DW_ENERGY, energy - 1)
            idx += 1

    return {
        "idx": idx,
        "respawned": respawned,
        "slot_epd": slot_epd,
        "hp": hp,
        "energy": energy,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 09 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
