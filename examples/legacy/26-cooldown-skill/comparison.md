# Scenario 26: Cooldown Skill System — Architecture Comparison

## 1. Scenario Intent
Implements an ability cooldown mechanics for Player 1:
- Ability ("Psionic Burst") triggers when a Ghost enters cast location 3.
- When ready, kills 2 enemy Zerglings, increments casts and kills, and begins a 300-frame cooldown.
- While on cooldown, cast attempts fail and increment the `wasted` counter.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Manual frame-by-frame countdown decrements
if EUDIf()(cooldown >= 1):
    cooldown -= 1
EUDEndIf()

if EUDIf()(Bring(Player1, AtLeast, 1, UNIT_GHOST, LOC_CAST_ZONE)):
    if EUDIf()(cooldown == 0):
        DoActions(KillUnitAt(BURST_KILLS, UNIT_ZERGLING, LOC_CAST_ZONE, Player8))
        kills_credited += BURST_KILLS
        casts += 1
        cooldown << COOLDOWN_LOOPS
    if EUDElse()():
        wasted += 1
    EUDEndIf()
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Declarative reactive delay with after()
if cooling == 1:
    with after(COOLDOWN_LOOPS - 1):
        ready = 1
        cooling = 0

if Bring(Player1, AtLeast, 1, "Terran Ghost", LOC_CAST_ZONE):
    if cooling == 1:
        wasted += 1
    if ready == 1:
        SetDeaths(Player8, Subtract, 2, "Zerg Zergling")
        kills_credited += 2
        casts += 1
        ready = 0
        cooling = 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 12 T | 21 T | **11 T** | **-8% vs Legacy, -48% vs Proto (v2 Wins)** |
| **Payload Size** | ~5,000 bytes | ~8,000 bytes | **3,896 bytes** | **-22% Payload** |
| **VProc Allocation** | 4 VProcs (288 B) | Dynamic | **0 VProcs** | **100% ByteBuf lowering** |
| **Timer Model** | Manual countdown var | `after(299)` | `with after(299):` | Zero manual timer loops |

---

## 4. Architectural Mechanics

### Reactive Delay Primitives
- Legacy code forces the developer to declare a `cooldown` variable and write per-frame decrement boilerplate (`if cooldown >= 1: cooldown -= 1`).
- Modern `eudkit` expresses the recovery using `with after(299):`. The compiler manages the timer triggers internally without manual countdown subtraction.

### Zero-VProc Lowering
- All bookkeeping metrics (`casts`, `wasted`, `kills_credited`, `ready`, `cooling`) are lowered into 4-byte ByteBuf memory cells, resulting in **0 VProcs** and beating legacy trigger consumption.
