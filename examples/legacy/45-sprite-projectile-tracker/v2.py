# 45 - sprite / projectile tracking (modern eudkit)
#
# Expresses ideal intent:
# 1. Native linked-list traversal on engine sprite list (0x629D98) with direct EPD access.
# 2. Integer centroid calculation (//) and cluster tracking with typed session variables.
# 3. Clean local session state with zero global singletons.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

SPRITE_LIST_HEAD = 0x629D98
DEATHS_BASE = 0x58A364
ARENA_MIN = 512
ARENA_MAX = 3584


def build(session: eudkit.Session):
    sprite_count = session.var(0)
    sum_x = session.var(0)
    sum_y = session.var(0)
    centroid_x = session.var(0)
    centroid_y = session.var(0)
    prev_count = session.var(0)
    appeared_total = session.var(0)
    largest_cluster = session.var(0)

    curr_ptr = session.var(0)
    curr_epd = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal sprite_count, sum_x, sum_y, centroid_x, centroid_y
        nonlocal prev_count, appeared_total, largest_cluster, curr_ptr, curr_epd

        sprite_count = 0
        sum_x = 0
        sum_y = 0

        curr_ptr = eudkit.dwread(SPRITE_LIST_HEAD)
        while curr_ptr != 0:
            curr_epd = (curr_ptr - DEATHS_BASE) // 4
            x = eudkit.wread_epd(curr_epd + 3, 0)
            y = eudkit.wread_epd(curr_epd + 3, 1)

            if x >= ARENA_MIN:
                if x <= ARENA_MAX:
                    if y >= ARENA_MIN:
                        if y <= ARENA_MAX:
                            sprite_count += 1
                            sum_x += x
                            sum_y += y

            curr_ptr = eudkit.dwread_epd(curr_epd + 1)

        if sprite_count > 0:
            centroid_x = sum_x // sprite_count
            centroid_y = sum_y // sprite_count

        if sprite_count > prev_count:
            appeared_total += sprite_count - prev_count
            if sprite_count > largest_cluster:
                largest_cluster = sprite_count
        prev_count = sprite_count

    return {
        "sprite_count": sprite_count,
        "centroid_x": centroid_x,
        "centroid_y": centroid_y,
        "largest_cluster": largest_cluster,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 45 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
