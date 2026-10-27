# Scenario 03: Block Copy (CUnit Snapshot) — Architecture Comparison

## 1. Scenario Intent
Copies an entire 84-dword (336-byte) CUnit structure from a dynamic source unit pointer into an allocated snapshot buffer on demand (`request_flag == 1`).
After completing the memory copy:
- Reads back the copied `unitType` field (offset +0x64 = +25 dwords) to verify integrity.
- Updates bookkeeping counters (`snapshots_done`, `last_source_epd`).
- Clears request and busy locks.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Raw Db allocation and macro-based memory mover
snapshot_buf = Db(336)
snapshot_epd = EPD(snapshot_buf)
...
if EUDIf()(request_flag == 1):
    if EUDIf()(copy_busy == 0):
        copy_busy << 1
        # Calls f_repmovsd_epd which hooks into global CP cursor engine
        f_repmovsd_epd(snapshot_epd, source_epd, SNAPSHOT_DWORDS)
        snapshot_type << f_dwread_epd(snapshot_epd + UNIT_TYPE_EPD_OFFSET)
        last_source_epd << source_epd
        snapshots_done += 1
        request_flag << 0
        copy_busy << 0
    EUDEndIf()
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Structured Array buffer with native while copy loop
snapshot_buf = Array(SNAPSHOT_DWORDS, epd=SNAPSHOT_BUF_EPD, session=session)
...
if request_flag == 1:
    if copy_busy == 0:
        copy_busy = 1
        i = 0
        while i < SNAPSHOT_DWORDS:
            dwwrite_epd(SNAPSHOT_BUF_EPD + i, dwread_epd(source_epd + i))
            i += 1

        snapshot_type = dwread_epd(SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET)
        last_source_epd = source_epd
        snapshots_done += 1
        request_flag = 0
        copy_busy = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 16 T (call-site delta) | 114 T (standalone) | **7.12x** | See Engine Coupling analysis below |
| **Global Hidden Engine** | ~80 T ReadTable + CP engine | 0 T (zero hidden dependencies) | — | Legacy requires global runtime table |
| **True Total Map Footprint** | ~96 T + injector glue | 114 T | **1.18x** | Negligible gap in actual map binaries |
| **VProc Allocation** | 6 VProcs (432 B) | 5 VProcs (scratch reused) | **0.83x** | -17% VProcs |
| **Buffer Abstraction** | Untyped `Db(336)` | Typed `Array` with bounds tracking | — | Type safe & memory protected |

---

## 4. Architectural Mechanics

### Memory Model & Buffer Abstraction
- **Legacy v1 (Raw Byte Blob)**: Legacy allocates memory with `Db(336)`. It returns an unmanaged address pointer with no type metadata, no index operations, and no bounds validation. Developers must manually compute byte-to-dword shifts (`EPD(snapshot_buf)`) and pass raw offsets.
- **Modern v2 (Typed Memory Module)**: `eudkit` encapsulates buffer storage in a first-class `Array` interface. Memory is accessed via structured `dwread_epd` / `dwwrite_epd` primitives or indexing syntax (`arr[i]`), ensuring clear ownership semantics.

### Seam Coupling vs Standalone Locality
- Legacy's 16-trigger call site is an illusion of brevity: `f_repmovsd_epd` relies on mutating global CurrentPlayer states and modifying pointers inside the shared import-time `ReadTable`. The module leaks side effects into engine CP state.
- Modern `eudkit` implements the block copy with pure AST loop lowering. The loop emits self-contained trigger sequences that preserve StarCraft engine invariants without hijacking global CP state or requiring fragile trigger-relink tricks (`SetNextPtr`).
