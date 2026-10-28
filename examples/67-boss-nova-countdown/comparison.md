# 67 - Boss Nova Countdown Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Manual Countdown Clocks & Raw Unit Loop Boilerplate
In legacy eudplib (`legacy.py`), telegraphing an arena blast with countdown stages and sound effects was complex:
- **Manual Warning Dispatch**: Stages 3, 2, and 1 were scheduled through independent `EUDIf` tests comparing `cycle_t >= CYCLE - N * WARN_STEP`.
- **String Table Boundary Friction**: Using `PlayWAV` required loading the map at module level to resolve sound strings.
- **Leaking Unit Structure Layout**: Accessing CUnit fields required manual offset calculations (`uepd + 19` for owner, `uepd + 10` for positions, `uepd + 2` for HP).

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Typed Actions & Scoped Traversal
In modern eudkit (`v2.py`), countdown and blast execution are structured cleanly:
- **First-Class `PlayWAV` Action**: Native typed `eudkit.PlayWAV` action integrated into the IR pipeline.
- **Direct CUnit Pointer Iteration**: Traversing `firstVisibleUnit` with sub-dword EPD indexing operates with minimal VProcs.
- **Deduplicated Map Strings**: Warning countdown texts ("3...", "2...", "1...") reside in the map string table without dynamic allocations.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~180 | **135** | **-25.0%** |
| **VProcs** | 16 | **10** | **-37.5%** |
| **Payload Size** | ~50,000 bytes | **37,992 bytes** | **-24.0%** |
| **State Locality** | 7 global singletons | Scoped session variables | Clean module boundary |

All budget gates pass:
- `v2_triggers (135) <= v1_triggers (~180)`
- `v2_payload_bytes (37,992) <= v1_payload_bytes (50,000)`

---

## 4. Architectural Mechanics

### Module Depth & Interface Design
- **Legacy v1 (Shallow Seam)**: The legacy implementation leaks low-level StarCraft engine memory structures across shallow call interfaces, requiring manual EPD arithmetic, raw memory pointers, and auxiliary scratch variables.
- **Modern eudkit v2 (Deep Module)**: Modern eudkit exposes a deep interface where complex trigger packing, VProc assignment, and IR lowering are encapsulated behind clean, type-safe seams.

### Locality & Compiler Leverage
- **State Locality**: Encapsulating variables within isolated compilation sessions preserves state locality and prevents cross-module global namespace pollution.
- **Compiler Leverage**: Modern eudkit leverages typed AST lowering, reactive rules (`once`, `every`, `when`), and stock trigger primitives to maximize execution efficiency while minimizing static payload size.
