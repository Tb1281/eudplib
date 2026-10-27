"""Scenario 23: Rank Ladder and Action Button Relabeling — Modern eudkit (v2).

Ideal Intent:
Relabel 7 rank titles and customize the command card attack button at game start:
- Recruit (1302) -> "Rookie"
- Corporal (1306) -> "Veteran"
- Sergeant (1308) -> "Drill Sergeant"
- Master Sergeant (1310) -> "First Sergeant"
- Captain (1312) -> "Commander"
- Colonel (1316) -> "Wing Commander"
- Admiral (1324) -> "Fleet Admiral"
- Attack Button (1400, offset 2) -> "Charge!"
Executes once on map startup using packed dwords into `stat_txt.tbl` (`0x6D5A30`).
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwwrite_epd,
    epdread_epd,
    once,
    wread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

# (tbl_id, byte_offset, new_text)
BUTTON_RENAMES = [
    (1302, 0, "Rookie"),
    (1306, 0, "Veteran"),
    (1308, 0, "Drill Sergeant"),
    (1310, 0, "First Sergeant"),
    (1312, 0, "Commander"),
    (1316, 0, "Wing Commander"),
    (1324, 0, "Fleet Admiral"),
    (1400, 2, "Charge!"),
]

PACKED_BUTTONS = []
for tid, off, name in BUTTON_RENAMES:
    b = name.encode("utf-8") + b"\x00"
    pad = (4 - len(b) % 4) % 4
    b += b"\x00" * pad
    dwords = [int.from_bytes(b[i : i + 4], "little") for i in range(0, len(b), 4)]
    PACKED_BUTTONS.append((tid, off, dwords))


def build(session: eudkit.Session):
    tbl_epd = session.var(0, name="tbl_epd")
    str_offset = session.var(0, name="str_offset")
    dst_epd = session.var(0, name="dst_epd")
    relabeled = session.var(0, name="relabeled")

    @session.fn
    def game_loop():
        with once():
            tbl_epd = epdread_epd(EPD(0x6D5A30))

            for i in range(len(PACKED_BUTTONS)):
                entry = PACKED_BUTTONS[i]
                str_offset = wread_epd(tbl_epd + (entry[0] // 2), (entry[0] % 2))
                dst_epd = tbl_epd + ((str_offset + entry[1]) // 4)

                for j in range(len(entry[2])):
                    dwwrite_epd(dst_epd + j, entry[2][j])

            relabeled = 1

    return {
        "relabeled": relabeled,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 23 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
