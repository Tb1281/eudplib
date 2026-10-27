# Scenario 22: Unit Type Renaming (TBL Patching) — Architecture Comparison

## 1. Scenario Intent
Renames 8 core combat units across all three races on game initialization:
- Marine -> "Space Marine"
- Ghost -> "Shadow Operative"
- Firebat -> "Pyro Specialist"
- Medic -> "Field Surgeon"
- Zergling -> "Voidling"
- Hydralisk -> "Spine Beast"
- Zealot -> "Blade Adept"
- Dragoon -> "War Strider"
Patches strings into the engine's `stat_txt.tbl` table (`0x6D5A30`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Calls f_settblf with hand-computed IDs and explicit format string
if EUDIf()(renamed == 0):
    f_settblf(TBL_MARINE, 0, "{}", "Space Marine", encoding="UTF-8")
    f_settblf(TBL_GHOST, 0, "{}", "Shadow Operative", encoding="UTF-8")
    f_settblf(TBL_FIREBAT, 0, "{}", "Pyro Specialist", encoding="UTF-8")
    f_settblf(TBL_MEDIC, 0, "{}", "Field Surgeon", encoding="UTF-8")
    ...
    renamed << 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Unrolled compile-time dword chunking inside reactive once() block
with once():
    tbl_epd = epdread_epd(EPD(0x6D5A30))
    for i in range(len(PACKED_RENAMES)):
        entry = PACKED_RENAMES[i]
        str_offset = wread_epd(tbl_epd + (entry[0] // 2), (entry[0] % 2))
        dst_epd = tbl_epd + (str_offset // 4)
        for j in range(len(entry[1])):
            dwwrite_epd(dst_epd + j, entry[1][j])
    renamed = 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 217 T | **BLOCKED** (serde schema bug) | **219 T** | **Unblocked, matches legacy parity** |
| **String Packing** | 1 byte / write | Prototype failure | **4 bytes / dword write** | **4x write leverage** |
| **Initialization Guard** | Manual flag check | N/A | Declarative `with once():` | Self-deactivating |
| **Global Hidden Engine** | 80 T ReadTable | N/A | 0 T | Standalone IR |

---

## 4. Architectural Mechanics

### Overcoming Prototype Serde Failure
- In the early research benchmark (`ours.js`), this scenario was completely **BLOCKED**: `tbl.ts` emitted variable declarations with Const expressions in place of Value nodes, crashing the Rust AST visitor.
- Modern `eudkit` solves this cleanly by pre-packing UTF-8 strings into contiguous 4-byte little-endian integers at build time. The resulting writes are lowered as standard `dwwrite_epd` instructions within a compile-time unrolled `range` loop, generating **219 triggers** and matching legacy's 217-trigger footprint.

### Write Leverage via Dword Packing
- Legacy `f_settblf` executes byte-by-byte formatting trampolines at runtime.
- Modern `eudkit` groups 4 ASCII characters into single 32-bit dwords, writing up to 4 characters per trigger action and dramatically accelerating game initialization speed.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
