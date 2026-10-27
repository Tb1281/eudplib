"""Example 04: Memory & Game Structs — Legacy eudplib (v1).

Demonstrates:
- Raw memory offsets and EPD address calculations (0x59CCA8 // 4)
- Repetitive bit-read triggers emitted by f_dwread_epd / f_wread_epd / f_bread_epd
- Lack of trigger deduplication across reading operations
- Manual word/byte masking and shifting
"""

from pathlib import Path
import eudplib as ep

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
BASEMAP_PATH = REPO_ROOT / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"


def main():
    ep.LoadMap(str(BASEMAP_PATH))
    ep.CompressPayload(True)

    def v1_memory_workload():
        # SC unit table pointer: 0x59CCA8
        cunit_epd = ep.EPD(0x59CCA8)

        # Each f_dwread_epd call expands a dedicated 32-bit binary search reading ladder
        # (32 triggers per call unless optimized by hand!)
        hp = ep.f_dwread_epd(cunit_epd + 2)
        pos = ep.f_dwread_epd(cunit_epd + 10)
        unit_id = ep.f_wread_epd(cunit_epd + 25, 0)
        player = ep.f_bread_epd(cunit_epd + 19, 1)

        # Writing memory requires f_dwwrite_epd
        if ep.EUDIf()(hp < 100):
            ep.f_dwwrite_epd(cunit_epd + 2, 500)
            ep.f_dwwrite_epd(cunit_epd + 10, 0x02000200)
        ep.EUDEndIf()

    out_path = Path(__file__).parent / "output_v1.scx"
    ep.SaveMap(str(out_path), v1_memory_workload)
    print(f"Legacy v1 memory workload compiled to: {out_path}")


if __name__ == "__main__":
    main()
