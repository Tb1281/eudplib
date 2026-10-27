# Scenario 21: Existing vs Completed Unit Counts — Architecture Comparison

## 1. Scenario Intent
Answers four population questions for Player 1:
- Existing Marines >= 1 -> flag 208 (`Command`).
- Existing Siege Tanks >= 3 -> flag 209 (`Command`).
- Completed Marines >= 1 -> flag 210 (`Memory` query at `0x584DE4`).
- Completed Siege Tanks >= 3 -> flag 211 (`Memory` query at `0x584DE4 + 240`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
if EUDIf()(Command(Player1, AtLeast, 1, MARINE)):
    DoActions([SetDeaths(Player1, SetTo, 1, 208)])
EUDEndIf()
if EUDIf()(Command(Player1, AtLeast, 3, SIEGE_TANK)):
    DoActions([SetDeaths(Player1, SetTo, 1, 209)])
EUDEndIf()
if EUDIf()(Memory(counts_addr(COMPLETED_BASE, P1, MARINE), AtLeast, 1)):
    DoActions([SetDeaths(Player1, SetTo, 1, 210)])
EUDEndIf()
if EUDIf()(Memory(counts_addr(COMPLETED_BASE, P1, SIEGE_TANK), AtLeast, 3)):
    DoActions([SetDeaths(Player1, SetTo, 1, 211)])
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
if Command(Player1, AtLeast, 1, "Terran Marine"):
    SetDeaths(Player1, SetTo, 1, 208)

if Command(Player1, AtLeast, 3, "Terran Siege Tank (Tank Mode)"):
    SetDeaths(Player1, SetTo, 1, 209)

if Memory(counts_addr(COMPLETED_BASE, 0, 0), AtLeast, 1):
    SetDeaths(Player1, SetTo, 1, 210)

if Memory(counts_addr(COMPLETED_BASE, 0, 5), AtLeast, 3):
    SetDeaths(Player1, SetTo, 1, 211)
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 12 T | **4 T** | **0.33x** | **-67% Triggers (v2 Wins)** |
| **Payload Size** | ~4,800 bytes | **2,600 bytes** | **0.54x** | **-46% Payload** |
| **VProc Allocation** | 0 VProcs | **0 VProcs** | — | Clean condition lowering |
| **String Lookup** | Manual numeric IDs | Symbolic unit names | — | High developer leverage |

---

## 4. Architectural Mechanics

### Trigger Inlining & Condition Packing
- In legacy v1, each check wraps a `DoActions` block inside an `EUDIf() ... EUDEndIf()`, creating overhead triggers for jump targets.
- Modern `eudkit` collapses each independent condition-action pair into a single native StarCraft trigger containing 1 condition and 1 action. Four queries compile to exactly **4 triggers**.

### Symbolic Name Resolution
- Legacy maps hardcode magic unit IDs (`MARINE = 0`, `SIEGE_TANK = 5`) because legacy condition constructors lacked basemap-aware string table resolvers.
- Modern `eudkit` accepts descriptive strings (`"Terran Marine"`, `"Terran Siege Tank (Tank Mode)"`) and resolves them deterministically at compile time through `MapContext`.
