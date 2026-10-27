# Scenario 23: Rank Ladder and Action Button Relabeling — Architecture Comparison

## 1. Scenario Intent
Patches UI text strings in StarCraft's `stat_txt.tbl` table at game start:
- 7 Player Rank labels: Recruit -> "Rookie", Corporal -> "Veteran", Sergeant -> "Drill Sergeant", Master Sergeant -> "First Sergeant", Captain -> "Commander", Colonel -> "Wing Commander", Admiral -> "Fleet Admiral".
- 1 Action button label: sets the attack button (ID 1400, offset +2) to "Charge!".

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
if EUDIf()(relabeled == 0):
    f_settblf(TBL_RANK_RECRUIT, 0, "{}", "Rookie", encoding="UTF-8")
    f_settblf(TBL_RANK_CORPORAL, 0, "{}", "Veteran", encoding="UTF-8")
    ...
    f_settblf(TBL_BUTTON_SET, BUTTON_TEXT_OFFSET, "{}", "Charge!", encoding="UTF-8")
    relabeled << 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
with once():
    tbl_epd = epdread_epd(EPD(0x6D5A30))
    for i in range(len(PACKED_BUTTONS)):
        entry = PACKED_BUTTONS[i]
        str_offset = wread_epd(tbl_epd + (entry[0] // 2), (entry[0] % 2))
        dst_epd = tbl_epd + ((str_offset + entry[1]) // 4)
        for j in range(len(entry[2])):
            dwwrite_epd(dst_epd + j, entry[2][j])
    relabeled = 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 217 T | **BLOCKED** (serde schema bug) | **207 T** | **-5% vs Legacy (v2 Wins)** |
| **Payload Size** | ~82,000 bytes | N/A | **78,144 bytes** | **-5% Payload** |
| **Initialization Guard** | Manual flag comparison | N/A | Declarative `with once():` | Self-deactivating |
| **Global Hidden Engine** | 80 T ReadTable | N/A | 0 T | Zero hidden runtime baggage |

---

## 4. Architectural Mechanics

### Overcoming Prototype Blocking
- Like Scenario 22, this scenario was BLOCKED in early research benchmarks due to AST schema serialization mismatches in the JavaScript compiler.
- Modern `eudkit` solves this cleanly with build-time dword packing and compile-time unrolled range loops, lowering to **207 triggers** and beating legacy's 217-trigger footprint.

### Offset Skipping Precision
- Modifying command card buttons requires skipping a 2-byte prefix (`<Key><01>`). Legacy `f_settblf` passed offset 2 into runtime formatters.
- Modern `eudkit` directly incorporates the byte offset into target address calculation `(str_offset + entry[1]) // 4`, allowing clean in-place replacement without breaking button hotkeys.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
