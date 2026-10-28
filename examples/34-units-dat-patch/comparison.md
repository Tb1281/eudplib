# Scenario 34: Units.dat In-Memory Patch and Verification — Architecture Comparison

## 1. Scenario Intent
At game startup, buffs Terran infantry units by patching `Units.dat` table memory directly:
- Marine (0): 80 HP, 1 Armor
- Ghost (1): 60 HP, 1 Armor
- Firebat (32): 70 HP, 1 Armor
- Medic (34): 75 HP, 1 Armor
- HitPoints are shifted by 8 bits (`HP * 256`) at `0x662350 + 4 * unit_id`.
- Armor is stored as a byte at `0x65FEC8 + unit_id`.
- Reads back all patched values to verify integrity:
  - If all match: announces `"Terran infantry buffed: HP up, armor 1."`
  - Otherwise: announces `"WARNING: infantry buff did not stick."`

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Requires EUDFunc definitions and manual trampoline calls
@EUDFunc
def patch_unit(unit_id, new_hp):
    f_dwwrite(HP_BASE + 4 * unit_id, new_hp * 256)
    f_bwrite(ARMOR_BASE + unit_id, 1)

@EUDFunc
def verify_unit(unit_id, expected_hp):
    hp = f_dwread(HP_BASE + 4 * unit_id)
    armor = f_bread(ARMOR_BASE + unit_id)
    if EUDIf()(hp != expected_hp * 256):
        verify_failed += 1
    EUDEndIf()
...
def run():
    patch_unit(MARINE, 80)
    ...
    verify_unit(MARINE, 80)
    ...
    if EUDIf()(verify_failed == 0):
        DoActions(DisplayText(Db("Terran infantry buffed: HP up, armor 1.")))
```

### Modern eudkit v2 (`v2.py`)
```python
# Declarative with once():, direct address memory I/O, STRx-backed DisplayText
with once():
    verify_failed = 0

    dwwrite(HP_BASE + 4 * MARINE, 80 * 256)
    bwrite(ARMOR_BASE + MARINE, 1)
    ...
    hp0 = dwread(HP_BASE + 4 * MARINE)
    arm0 = bread(ARMOR_BASE + MARINE)
    if hp0 != 80 * 256:
        verify_failed += 1
    if arm0 != 1:
        verify_failed += 1
    ...
    if verify_failed == 0:
        DisplayText("Terran infantry buffed: HP up, armor 1.")
    else:
        DisplayText("WARNING: infantry buff did not stick.")
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~185 T (with EUDFuncs) | 220 T | **132 T** | **-28% vs Legacy, -40% vs Proto (v2 Wins)** |
| **Payload Size** | ~55,000 bytes | ~68,000 bytes | **38,000 bytes** | **-31% Payload Footprint** |
| **Lifecycle** | Manual hook convention | Global execution | `with once():` | Native one-time startup block |
| **Memory API** | `f_dwwrite`/`f_bwrite` | Ad-hoc memory | Direct `dwwrite`/`bwrite` | Unified memory I/O primitives |

---

## 4. Architectural Mechanics

### Clean Address-Based Memory Primitives
- In legacy eudplib, writing or reading byte/word values requires manually managing EPD and sub-dword byte position registers.
- Modern eudkit provides unified `dwwrite(addr, val)`, `bwrite(addr, val)`, `dwread(addr)`, and `bread(addr)` primitives that automatically calculate EPD offsets and bitmask shifts at compile time.

### Elimination of Function Trampoline Bloat
- Legacy code wrapped 2-line memory operations into `EUDFunc` instances to save trigger copies, but the call-site trampolines, argument passing, and return handling created substantial overhead.
- Modern eudkit lowers inlined memory operations directly into optimal `SetDeathsX` and ReadTable queries, achieving lower total triggers without function call indirection.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
