"""Scenario 12: Player Nickname Reader — Modern eudkit (v2).

Ideal Intent:
Read the first 4 bytes of a player's nickname from the player name table (`0x57EEE8`):
- Each player record is 36 bytes (9 dwords).
- The player nickname string starts at offset +3 bytes into the record.
- Byte 0 is at `base_epd, subp 3`.
- Bytes 1..3 are at `base_epd + 1, subp 0..2`.
- Pack the 4 characters into a composite `name_sum` dword.
- Set `name_flag = 1` if the name starts with uppercase 'A' (`0x41`), else `0`.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    bread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

NAME_TABLE_EPD = EPD(0x57EEE8)
STRIDE_DWORDS = 9  # 36 bytes / 4


def build(session: eudkit.Session):
    target_player = session.var(1, name="target_player")
    name_flag = session.var(0, name="name_flag")
    name_sum = session.var(0, name="name_sum")
    base_epd = session.var(0, name="base_epd")
    b0 = session.var(0, name="b0")
    b1 = session.var(0, name="b1")
    b2 = session.var(0, name="b2")
    b3 = session.var(0, name="b3")

    @session.fn
    def game_loop():
        # Clean dword-aligned base address calculation
        base_epd = NAME_TABLE_EPD + target_player * STRIDE_DWORDS

        # Extract 4 sequential name characters across dword boundaries
        b0 = bread_epd(base_epd, 3)
        b1 = bread_epd(base_epd + 1, 0)
        b2 = bread_epd(base_epd + 1, 1)
        b3 = bread_epd(base_epd + 1, 2)

        # Pack into 32-bit checksum
        name_sum = b0 + (b1 << 8) + (b2 << 16) + (b3 << 24)

        # First character validation
        if b0 == 0x41:  # 'A'
            name_flag = 1
        else:
            name_flag = 0

    return {
        "target_player": target_player,
        "name_flag": name_flag,
        "name_sum": name_sum,
        "base_epd": base_epd,
        "b0": b0,
        "b1": b1,
        "b2": b2,
        "b3": b3,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 12 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
