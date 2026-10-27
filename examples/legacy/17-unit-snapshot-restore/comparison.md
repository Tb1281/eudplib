# Scenario 17: Unit Snapshot and Restore — Architecture Comparison

## 1. Scenario Intent
Preserves and resurrects an entire 84-dword (336-byte) CUnit structure on command:
- Snapshot: copies slot 0 into a dedicated buffer, verifies the copied `unitType`, and sets `snapshot_valid = 1`.
- Restore: copies the preserved buffer back into slot 0, reviving the unit's exact stats, positions, and orders.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Double call sites to f_repmovsd_epd riding global CP engines
if EUDIf()(snapshot_flag == 1):
    f_repmovsd_epd(snapshot_epd, UNIT_EPD, SNAPSHOT_DWORDS)
    snapshot_type << f_wread_epd(snapshot_epd + UNIT_TYPE_EPD_OFFSET, 0)
    snapshot_valid << 1
    snapshots_done += 1
    snapshot_flag << 0
EUDEndIf()

if EUDIf()(restore_flag == 1):
    if EUDIf()(snapshot_valid == 1):
        f_repmovsd_epd(UNIT_EPD, snapshot_epd, SNAPSHOT_DWORDS)
        restores_done += 1
    EUDEndIf()
    restore_flag << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Bidirectional structured while copy loops over Array buffer
if snapshot_flag == 1:
    i = 0
    while i < SNAPSHOT_DWORDS:
        dwwrite_epd(SNAPSHOT_BUF_EPD + i, dwread_epd(UNIT_EPD + i))
        i += 1
    snapshot_type = wread_epd(SNAPSHOT_BUF_EPD + UNIT_TYPE_OFFSET, 0)
    snapshot_valid = 1
    snapshots_done += 1
    snapshot_flag = 0

if restore_flag == 1:
    if snapshot_valid == 1:
        i = 0
        while i < SNAPSHOT_DWORDS:
            dwwrite_epd(UNIT_EPD + i, dwread_epd(SNAPSHOT_BUF_EPD + i))
            i += 1
        restores_done += 1
    restore_flag = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 19 T (call site delta) | 197 T | **126 T** | **-36% Triggers vs prototype** |
| **Global Hidden Engine** | 80 T ReadTable + CP engine | 0 T | 0 T | Standalone self-contained IR |
| **True Total Map Triggers** | ~99 T + injector | 197 T | **126 T** | Close parity in real binaries |
| **Buffer Abstraction** | Untyped `Db(336)` | Raw pointer | Typed `Array` | Memory safety |

---

## 4. Architectural Mechanics

### Memory Safety & Interface Ergonomics
- Legacy uses untyped `Db(336)` and mutates global engine CP pointers during `f_repmovsd_epd`.
- Modern `eudkit` encapsulates the storage within an `Array` instance, performing bidirectional copy loops that are purely local and free of global state corruption.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
