# 68 - save code encode + display (modern eudkit)
#
# Expresses ideal intent:
# 1. Consolidated `StringBuffer` API for single dynamic string architecture (ADR 0004).
# 2. Reactive rules (`with session.once():`, `with session.every(120):`) eliminating manual clocks.
# 3. Packed progression encoding and check digit generation with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    code_buf = session.string_buffer(64)

    level = session.var(0)
    stage = session.var(0)
    gold_tier = session.var(0)
    boss_flags = session.var(0)
    next_flag_bit = session.var(0)
    code_body = session.var(0)
    check = session.var(0)
    save_code = session.var(0)
    prints = session.var(0)
    codes_shown = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal level, stage, gold_tier, boss_flags, next_flag_bit
        nonlocal code_body, check, save_code, prints, codes_shown

        with session.once():
            next_flag_bit = 1

        with session.every(120):
            stage += 1
            if stage >= 10:
                stage = 0
                if level < 20:
                    level += 1

            if stage % 3 == 0 and stage >= 1:
                boss_flags = boss_flags | next_flag_bit
                next_flag_bit = next_flag_bit * 2

            code_body = ((level * 10 + stage) * 10 + gold_tier) * 1000 + boss_flags
            check = (level * 3 + stage * 5 + gold_tier * 7 + boss_flags * 11) % 10
            save_code = code_body * 10 + check

            code_buf.printf("SAVE CODE: ", save_code)
            code_buf.display()
            prints += 1
            codes_shown += 1

    return {
        "level": level,
        "stage": stage,
        "save_code": save_code,
        "prints": prints,
        "codes_shown": codes_shown,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 68 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
