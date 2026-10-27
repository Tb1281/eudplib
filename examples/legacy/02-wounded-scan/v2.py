"""Scenario 02: Wounded-Unit Scan — Modern eudkit (v2).

Ideal Intent:
Walk the active unit linked list every trigger cycle and census wounded units.
For each unit:
- Check unit HP (offset +0x08, EPD + 2 dwords in fixed-point 1/256 HP)
- Filter out mid-removal units (HP == 0)
- Count units below wounded threshold (< 50% HP = 5120)
- Count units below critical threshold (< 25% HP = 2560)
- Latch report variables and toggle an alarm state if any critical unit is present.
"""

from pathlib import Path
import eudkit
from eudkit import (
    EPD,
    dwread_epd,
    epdread_epd,
)

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

# HP thresholds in engine fixed-point units (HP * 256)
WOUNDED_MAX = 5120
CRITICAL_MAX = 2560


def build(session: eudkit.Session):
    scanned = session.var(0, name="scanned")
    wounded = session.var(0, name="wounded")
    critical = session.var(0, name="critical")
    report_scanned = session.var(0, name="report_scanned")
    report_wounded = session.var(0, name="report_wounded")
    report_critical = session.var(0, name="report_critical")
    alarm = session.var(0, name="alarm")

    cur_epd = session.var(0, name="cur_epd")
    cur_hp = session.var(0, name="cur_hp")

    @session.fn
    def game_loop():
        # Reset per-loop tallies
        scanned = 0
        wounded = 0
        critical = 0

        # Traverse active unit linked list (head pointer at 0x628430)
        cur_epd = epdread_epd(EPD(0x628430))
        while cur_epd != EPD(0):
            cur_hp = dwread_epd(cur_epd + 2)
            if cur_hp > 0:
                scanned += 1
                if cur_hp < WOUNDED_MAX:
                    wounded += 1
                    if cur_hp < CRITICAL_MAX:
                        critical += 1
            # Step to next unit in linked list (ptr at offset +4 bytes = +1 dword)
            cur_epd = epdread_epd(cur_epd + 1)

        # Latch census results
        report_scanned = scanned
        report_wounded = wounded
        report_critical = critical

        # Alarm trigger
        if critical >= 1:
            alarm = 1
        else:
            alarm = 0

    return {
        "scanned": scanned,
        "wounded": wounded,
        "critical": critical,
        "report_scanned": report_scanned,
        "report_wounded": report_wounded,
        "report_critical": report_critical,
        "alarm": alarm,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 02 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
