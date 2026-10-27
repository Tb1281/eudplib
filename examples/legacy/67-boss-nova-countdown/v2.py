# 67 - boss nova countdown (modern eudkit)
#
# Expresses ideal intent:
# 1. Staged telegraph countdown with native audio triggers via `PlayWAV`.
# 2. Area detonation with direct CUnit coordinate testing and HP reduction.
# 3. Deduplicated map strings and clean session state encapsulation.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

P1 = 0
CYCLE = 600
WARN_STEP = 24
NOVA_DAMAGE = 25600
ARENA_X0, ARENA_X1 = 1024, 3072
ARENA_Y0, ARENA_Y1 = 1024, 3072

FIRST_VISIBLE_UNIT = 0x628430
DEATHS_BASE = 0x58A364


def build(session: eudkit.Session):
    cycle_t = session.var(0)
    warn_stage = session.var(3)
    nova_count = session.var(0)
    warnings_shown = session.var(0)
    last_blast_hits = session.var(0)
    total_hits = session.var(0)

    uptr = session.var(0)
    uepd = session.var(0)
    scratch_hp = session.var(0)

    msg_3 = session.map_string("Boss nova in 3...")
    msg_2 = session.map_string("Boss nova in 2...")
    msg_1 = session.map_string("Boss nova in 1...")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal cycle_t, warn_stage, nova_count, warnings_shown
        nonlocal last_blast_hits, total_hits, uptr, uepd, scratch_hp

        cycle_t += 1

        if warn_stage == 3 and cycle_t >= CYCLE - 3 * WARN_STEP:
            eudkit.PlayWAV("sound\\glue\\bnetclick.wav")
            msg_3.display()
            warn_stage = 2
            warnings_shown += 1

        if warn_stage == 2 and cycle_t >= CYCLE - 2 * WARN_STEP:
            eudkit.PlayWAV("sound\\glue\\bnetclick.wav")
            msg_2.display()
            warn_stage = 1
            warnings_shown += 1

        if warn_stage == 1 and cycle_t >= CYCLE - WARN_STEP:
            eudkit.PlayWAV("sound\\glue\\bnetclick.wav")
            msg_1.display()
            warn_stage = 0
            warnings_shown += 1

        if cycle_t >= CYCLE:
            last_blast_hits = 0
            uptr = eudkit.dwread(FIRST_VISIBLE_UNIT)
            while uptr != 0:
                uepd = (uptr - DEATHS_BASE) // 4
                if eudkit.bread_epd(uepd + 19, 0) == P1:
                    x = eudkit.wread_epd(uepd + 10, 0)
                    if x >= ARENA_X0 and x <= ARENA_X1:
                        y = eudkit.wread_epd(uepd + 10, 1)
                        if y >= ARENA_Y0 and y <= ARENA_Y1:
                            scratch_hp = eudkit.dwread_epd(uepd + 2)
                            if scratch_hp > NOVA_DAMAGE:
                                eudkit.dwwrite_epd(uepd + 2, scratch_hp - NOVA_DAMAGE)
                            else:
                                eudkit.dwwrite_epd(uepd + 2, 0)
                            last_blast_hits += 1
                            total_hits += 1
                uptr = eudkit.dwread_epd(uepd + 1)

            nova_count += 1
            cycle_t = 0
            warn_stage = 3

    return {
        "cycle_t": cycle_t,
        "nova_count": nova_count,
        "warnings_shown": warnings_shown,
        "last_blast_hits": last_blast_hits,
        "total_hits": total_hits,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 67 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
