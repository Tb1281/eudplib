# 48 - runtime buffer -> string table copy (modern eudkit)
#
# Expresses ideal intent:
# 1. Single dynamic string architecture: consolidated on modern StringBuffer (ADR 0004).
# 2. Replaces fragile runtime CP block copy (`f_repmovsd_epd`) with static STRx
#    relocation and residue-stride CPString updates.
# 3. Clean local session state.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    status_buf = session.string_buffer(64)

    wave = session.var(0)
    kills = session.var(0)
    gold = session.var(0)
    wave_timer = session.var(0)

    filled_wave = session.var(-1)
    filled_kills = session.var(-1)
    filled_gold = session.var(-1)
    filled_timer = session.var(-1)
    copies_done = session.var(0)
    line_version = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal filled_wave, filled_kills, filled_gold, filled_timer
        nonlocal copies_done, line_version

        stale = 0
        if filled_wave != wave:
            stale = 1
        if filled_kills != kills:
            stale = 1
        if filled_gold != gold:
            stale = 1
        if filled_timer != wave_timer:
            stale = 1

        if stale == 1:
            status_buf.printf("Wave %d | Kills %d | Gold %d | Timer %d", wave, kills, gold, wave_timer)
            status_buf.display_text()
            filled_wave = wave
            filled_kills = kills
            filled_gold = gold
            filled_timer = wave_timer
            copies_done += 1
            line_version += 1

    return {
        "status_buf": status_buf,
        "copies_done": copies_done,
        "line_version": line_version,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 48 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
