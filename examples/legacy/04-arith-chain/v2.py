"""Scenario 04: Arithmetic Expression Chain — Modern eudkit (v2).

Ideal Intent:
Calculate complex compound arithmetic expressions over game state variables:
1. Attack calculation: `((atk_base + atk_bonus) * atk_mul - armor_flat) // divisor % wrap_mod`
2. Cooldown timing: unpack high/low words from raw timer with bitwise shift and mask,
   derive phase ticks using integer division (`//`) and modulo (`%`).
3. Bounded accumulation loop: compute unrolled linear summation.
4. Threshold check: reset result when overflowing threshold.
"""

from pathlib import Path
import eudkit

REPO_ROOT = Path(__file__).resolve().parents[3]
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def build(session: eudkit.Session):
    atk_base = session.var(120, name="atk_base")
    atk_bonus = session.var(35, name="atk_bonus")
    atk_mul = session.var(3, name="atk_mul")
    armor_flat = session.var(17, name="armor_flat")
    divisor = session.var(4, name="divisor")
    wrap_mod = session.var(1000, name="wrap_mod")
    result = session.var(0, name="result")

    raw_timer = session.var(0x12345678, name="raw_timer")
    cd_ticks = session.var(0, name="cd_ticks")
    cd_phase = session.var(0, name="cd_phase")
    loop_guard = session.var(0, name="loop_guard")
    hi = session.var(0, name="hi")

    @session.fn
    def game_loop():
        # Chain 1: Native compound arithmetic expression (direct // and % lowering)
        result = ((atk_base + atk_bonus) * atk_mul - armor_flat) // divisor % wrap_mod

        # Chain 2: Bitwise shifts, bitwise AND/XOR, and mixed modulo
        hi = raw_timer >> 16
        cd_ticks = (hi * 7 + (raw_timer & 0xFFFF)) % 3600
        cd_phase = ((cd_ticks // 60) & 0xFF) ^ (cd_ticks % 60)

        # Chain 3: Structured bounded loop accumulation
        loop_guard = 0
        while loop_guard < 24:
            result += loop_guard * 3
            loop_guard += 1

        # Keep calculation active
        if result >= 500:
            result = 0

    return {
        "atk_base": atk_base,
        "atk_bonus": atk_bonus,
        "atk_mul": atk_mul,
        "armor_flat": armor_flat,
        "divisor": divisor,
        "wrap_mod": wrap_mod,
        "result": result,
        "raw_timer": raw_timer,
        "cd_ticks": cd_ticks,
        "cd_phase": cd_phase,
        "loop_guard": loop_guard,
        "hi": hi,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    print("Scenario 04 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
