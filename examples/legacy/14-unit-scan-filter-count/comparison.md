# Scenario 14: Unit Table Scan and Filter/Count — Architecture Comparison

## 1. Scenario Intent
Scans the 1700-slot unit table each frame to census game entity populations:
- Counts total active units on the battlefield (`unitType != 228`).
- Filters for units owned by Player 1 (`owner == 0`).
- Filters for Terran Marines (`unitType == 0`), tallying veteran Marines (`hp >= 256`) vs wounded Marines (`hp < 256`).
- Drives scoreboard notification flags via `SetDeaths`.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Macro loop with non-local EUDContinue skips
for cu in EUDLoopCUnit():
    if EUDIf()(cu.unitType == NONE_TYPE):
        EUDContinue()
    EUDEndIf()
    total_live += 1

    if EUDIf()(cu.owner != 0):
        EUDContinue()
    EUDEndIf()

    if EUDIf()(cu.unitType != MARINE):
        EUDContinue()
    EUDEndIf()
    p1_marines += 1

    if EUDIf()(cu.hp >= 256):
        p1_veterans += 1
    if EUDElse()():
        p1_wounded += 1
    EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Structured single-pass while loop with nested conditionals
i = 0
while i < 1700:
    slot_epd = CUNIT_BASE_EPD + i * STRIDE
    unit_type = wread_epd(slot_epd + 25, 0)
    if unit_type != NONE_TYPE:
        total_live += 1
        owner = bread_epd(slot_epd + 19, 0)
        if owner == 0:
            if unit_type == MARINE:
                p1_marines += 1
                hp = dwread_epd(slot_epd + 2)
                if hp >= 256:
                    p1_veterans += 1
                else:
                    p1_wounded += 1
    i += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 38 T (call site delta) | 92 T (two-pass sweep) | **147 T** (single unified pass) | See Engine Amortization note |
| **Pass Count** | 1 pass | 2 passes (census + marine) | **1 pass** | Full data locality |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Zero hidden dependencies |
| **Control Flow Idiom** | 3 `EUDContinue()` jumps | Functional lambdas | Structured nested `if` | Clean AST |

---

## 4. Architectural Mechanics

### Single-Pass Locality vs Dual-Sweep Overhead
- In the early JS research prototype, compiling the filter required two separate `EUDLoopCUnit` sweeps because the AST lowerer could not evaluate scalar owner comparisons within the same pass.
- Modern `eudkit` unifies the entire census into a single, cohesive `while i < 1700:` traversal. The engine reads `unitType`, checks `owner`, and inspects `hp` in a single pass over each slot, maintaining strict spatial and temporal cache locality.

### Control Transfer Seams
- Legacy code relied on early-skipping macros (`EUDContinue()`), rewriting StarCraft trigger next-pointers on every negative condition.
- Modern v2 expresses the filtering hierarchy as structured nested conditionals. Inactive slots bypass subsequent field reads naturally without trigger-pointer modification.
