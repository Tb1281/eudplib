# 48 - Runtime Buffer -> String Table Copy Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Fragile Memory Moves vs Static STRx Relocation
In legacy eudplib (`legacy.py`), updating a dynamic status line string at runtime relied on low-level memory block copying:
- **Raw `Db` Buffer Allocation**: Legacy allocated a raw 64-byte `Db` block and manually tracked dword offsets (`OFF_WAVE = 1`, `OFF_KILLS = 2`, etc.), requiring constant manual recalculation if the layout changed.
- **Dynamic Pointer Resolution (`f_epdread_epd`)**: Resolving string slot 7's address required reading the string offset pointer array at `0x191943C8` using `f_epdread_epd`.
- **Runtime Block Copy (`f_repmovsd_epd`)**: Copying the 16 dwords into game memory executed an engine CP-cursor memory copy loop, adding significant execution time and trigger complexity.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Consolidated `StringBuffer` Architecture (ADR 0004)
In modern eudkit (`v2.py`), dynamic string updating is unified under the single `StringBuffer` interface:
- **Single Dynamic String Architecture**: Per the resolution of Issue #27, ambiguous legacy string types (`EUDString`, `DBString`, `Db`) are consolidated on `StringBuffer`.
- **Residue-Stride CPString Updates**: Instead of allocating an auxiliary buffer and copying whole blocks via `repmovsd`, `status_buf.printf("...")` directly updates the relocated STRx string using residue-stride CPString triggers with **zero runtime memory moving loops**.
- **Dramatic Footprint Reduction**: Triggers drop from ~65 down to **20** (-69.2%), payload shrinks by over 60%, and the need for fragile pointer arithmetic is completely eliminated.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~65 | **20** | **-69.2%** |
| **VProcs** | 18 | **8** | **-55.6%** |
| **Payload Size** | ~28,000 bytes | **10,416 bytes** | **-62.8%** |
| **String Architecture** | Raw Db + repmovsd copy | Native `StringBuffer` (STRx) | High abstraction depth |

All budget gates pass:
- `v2_triggers (20) <= v1_triggers (~65)`
- `v2_payload_bytes (10,416) <= v1_payload_bytes (28,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
