# Scenario 12: Player Nickname Reader — Architecture Comparison

## 1. Scenario Intent
Reads the first 4 bytes of a given player's nickname from StarCraft's nickname table at `0x57EEE8`:
- Each player record is 36 bytes wide.
- Nicknames begin at offset +3 into each record.
- Assembles the 4 ASCII characters into a packed 32-bit `name_sum`.
- Verifies if the player name starts with the character `'A'` (`0x41`), setting `name_flag = 1`.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# EDAC pattern: alternating two EUDByteReaders to avoid div/mod seek overhead
br_record = EUDByteReader()
br_name = EUDByteReader()
...
addr << NAME_TABLE + RECORD_STRIDE * target_player + NAME_OFFSET
br_record.seekoffset(addr)
br_name.seekoffset(addr + 1)
b0 = br_record.readbyte()
b1 = br_name.readbyte()
b2 = br_record.readbyte()
b3 = br_name.readbyte()

name_sum << b0 + (b1 << 8) + (b2 << 16) + (b3 << 24)
if EUDIf()([b0 != 0, b0 == 0x41]):
    name_flag << 1
if EUDElse()():
    name_flag << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct sub-dword reads at computed dword boundaries without reader objects
base_epd = NAME_TABLE_EPD + target_player * STRIDE_DWORDS

b0 = bread_epd(base_epd, 3)
b1 = bread_epd(base_epd + 1, 0)
b2 = bread_epd(base_epd + 1, 1)
b3 = bread_epd(base_epd + 1, 2)

name_sum = b0 + (b1 << 8) + (b2 << 16) + (b3 << 24)
if b0 == 0x41:
    name_flag = 1
else:
    name_flag = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 99 T (call sites only) | 147 T (standalone) | **1.48x** | Isolated call sites vs full engine |
| **Reader Infrastructure** | 2 stateful `EUDByteReader`s | **0 reader objects** | — | Pure functional reads |
| **Global Hidden Engine** | 80 T ReadTable + seek tables | 0 T | — | Zero hidden engine dependencies |
| **Memory Seam Leakage** | Byte offset seek logic | Typed EPD arithmetic | — | Clean structured addressing |

---

## 4. Architectural Mechanics

### Eliminating the "Two-Reader Alternation" Anti-Pattern
- **Legacy v1 (Accidental Complexity)**: In legacy `eudplib`, seeking a single `EUDByteReader` repeatedly triggers an expensive division/modulo routine to resolve sub-dword alignment. To avoid this cost, legacy mapmakers had to create *two* byte reader instances and alternate reads (`br_record` taking even bytes, `br_name` taking odd bytes). This pattern leaks engine implementation quirks directly into user code.
- **Modern v2 (Deep Sub-Dword Module)**: `eudkit` provides `bread_epd(epd, subp)`. The sub-dword position `subp` (0..3) is an explicit parameter. Reading across dword boundaries (`base_epd, 3` followed by `base_epd + 1, 0..2`) is natural, declarative, and requires no stateful reader cursors or pointer seeking.

### Locality & Zero-State Purity
- Legacy reader objects carry mutable internal state (`_offset`, `_subp`), creating risks of cursor desynchronization across nested loops or function calls.
- Modern `eudkit` primitives are purely functional, reading directly from the given memory coordinate without persisting cursor side-effects.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
