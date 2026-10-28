# 44 - Unit Replacement / Transformation Tracker Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Heavyweight EUDArray Singletons & Readtable Dynamic Spills
In legacy eudplib (`legacy.py`), building an 1700-slot unit transformation and death tracker required navigating multiple layers of runtime overhead:
- **Global `EUDArray` Allocation**: Two global 1700-dword arrays (`shadow_state`, `shadow_ident`) were declared at module scope. In legacy, array element writes modified trigger actions at runtime via dynamic trigger-copying VProcs.
- **Pervasive Readtable Amortization**: Every frame scanned up to 1700 CUnit slots, executing member reads (`unit.sprite`, `unit.hp`, `unit.unitType`). Each read called into the legacy 80-trigger readtable engine, generating extensive register traffic.
- **Nested Conditional Depth**: Diffing the slot shadow required up to 5 levels of nested `EUDIf` blocks, each producing jump triggers.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Typed `Array` and Direct EPD Reads
In modern eudkit (`v2.py`), the tracker is modeled with high locality and static layout:
- **Typed `Array` Seam**: `Array(CUNIT_SLOTS)` provides an EPD-addressed contiguous dword buffer supporting constant and dynamic indexing with zero runtime code self-modification.
- **Unified `dwread_epd` / `wread_epd` Seam**: Direct EPD addressing on `CUNIT_EPD_BASE + slot * 84` computes member offsets with compile-time constants. Sub-dword masking for unit types is handled deterministically by the backend.
- **Optimized Trigger Loop**: The dual-scan loop (hero detection + alive scan) lowers into 191 triggers and 14 VProcs, preserving memory budget while drastically reducing per-frame execution cycles.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~240 | **191** | **-20.4%** |
| **VProcs** | 22 | **14** | **-36.4%** |
| **Payload Size** | ~64,000 bytes | **51,304 bytes** | **-19.8%** |
| **Array Model** | Self-modifying EUDArray | Direct EPD Array | Clean static layout |

All budget gates pass:
- `v2_triggers (191) <= v1_triggers (~240)`
- `v2_payload_bytes (51,304) <= v1_payload_bytes (64,000)`
