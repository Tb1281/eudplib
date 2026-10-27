# Scenario 06: Arbitrary-Mask Bit Checking — Architecture Comparison

## 1. Scenario Intent
Tests arbitrary bit patterns within an engine flags dword (`0x58A364 + 0x200`) each game loop:
- Bit 0 (`0x00000001` - Shield Ready)
- Bit 5 (`0x00000020` - Stealth Active)
- Bit 10 (`0x00000400` - Hero Aura)
- Drives four binary indicator variables (`lamp_shield`, `lamp_stealth`, `lamp_aura`, `lamp_any`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
flags = f_dwread_epd(FLAGS_EPD)

if EUDIf()(flags & MASK_SHIELD != 0):
    lamp_shield << 1
if EUDElse()():
    lamp_shield << 0
EUDEndIf()
...
if EUDIf()(flags & (MASK_SHIELD | MASK_STEALTH | MASK_AURA) != 0):
    lamp_any << 1
if EUDElse()():
    lamp_any << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
flags = dwread_epd(FLAGS_EPD)

if flags & MASK_SHIELD != 0:
    lamp_shield = 1
else:
    lamp_shield = 0
...
if flags & (MASK_SHIELD | MASK_STEALTH | MASK_AURA) != 0:
    lamp_any = 1
else:
    lamp_any = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 25 T (call sites only) | 104 T (self-contained) | **4.16x** | Isolated call sites vs full engine |
| **VProc Allocation** | 5 VProcs (360 B) | **0 VProcs** (0 B) | **0.00x** | **100% ByteBuf storage lowering** |
| **Global Hidden Engine** | 80 T (mandatory readtable) | 0 T | — | Zero hidden engine dependencies |
| **True Total Map Triggers** | 105 T + injector | 104 T | **0.99x** | **v2 wins in actual map binary** |

---

## 4. Architectural Mechanics

### Storage Lowering & VProc Elimination
- **Legacy v1 (Heavyweight Triggers)**: Every `EUDVariable` in legacy `eudplib` allocates a dedicated 72-byte StarCraft trigger (VProc) regardless of usage patterns. Five variables consume 360 bytes of permanent trigger space.
- **Modern v2 (Zero-VProc Storage Lowering)**: The `eudkit` compiler performs lifetime and mutability analysis. Variables like `lamp_shield` and `flags` that are only assigned constants or populated via direct memory writes are lowered to compact 4-byte `ByteBuf` memory cells. The result is **0 VProcs**, saving critical trigger slot limits in StarCraft maps.

### Control Flow Seam
- Legacy v1 requires verbose `if EUDIf() ... if EUDElse()() ... EUDEndIf()` macro constructs that make branch logic difficult to read and error-prone.
- Modern v2 lowers standard Python `if / else` syntax directly, optimizing consecutive if/else blocks into linear trigger conditions without manual trigger chaining.
