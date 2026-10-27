# 49 - large-table linear search with early exit (modern eudkit)
#
# Expresses ideal intent:
# 1. High-level Array database linear search with native `break` early exit.
# 2. Local session variables with zero global singletons.
# 3. Direct EPD-addressed array storage with optimal trigger lowerings.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

TABLE_N = 512
NOT_FOUND = -1


def build(session: eudkit.Session):
    item_key = eudkit.Array(TABLE_N)
    item_price = eudkit.Array(TABLE_N)

    search_key = session.var(0)
    search_request = session.var(0)
    found_index = session.var(NOT_FOUND)
    found_price = session.var(0)
    searches_done = session.var(0)
    not_found_count = session.var(0)
    total_probes = session.var(0)
    best_price = session.var(0x7FFFFFFF)

    idx = session.var(0)
    probes = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal search_request, found_index, found_price, searches_done
        nonlocal not_found_count, total_probes, best_price, idx, probes

        if search_request == 1:
            idx = 0
            probes = 0
            found_index = NOT_FOUND

            while idx < TABLE_N:
                probes += 1
                if item_key[idx] == search_key:
                    found_index = idx
                    found_price = item_price[idx]
                    break
                idx += 1

            if found_index == NOT_FOUND:
                not_found_count += 1
            else:
                if found_price < best_price:
                    best_price = found_price

            total_probes += probes
            searches_done += 1
            search_request = 0

    return {
        "item_key": item_key,
        "item_price": item_price,
        "searches_done": searches_done,
        "not_found_count": not_found_count,
        "best_price": best_price,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 49 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
