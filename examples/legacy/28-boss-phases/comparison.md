# Scenario 28: Boss Phase State Machine — Architecture Comparison

## 1. Scenario Intent
Implements an autonomous multi-phase boss fight with 3000 HP:
- **Phase 1 (>2000 HP)**: Marines in location 4 deal 10 damage per loop.
- **Phase 2 (<=2000 HP)**: Spawns 4 Hydralisks on transition; spawns 2 Zerglings every 100 loops.
- **Phase 3 (<=1000 HP)**: Spawns 6 Zerglings on transition; spawns 2 Zerglings + 1 Hydralisk every 60 loops with a minimap ping.
- **Phase 4 (==0 HP)**: Awards 1000 Ore to Player 1, pings boss zone, and latches victory.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Legacy variables cannot be initialized with initial non-zero constants
if EUDIf()(phase == 0):
    boss_hp << BOSS_MAX_HP
    phase << 1
EUDEndIf()

# Manual scratch variable recomputation and two-way inequality testing
new_phase << phase
if EUDIf()(boss_hp <= 2000):
    new_phase << 2
EUDEndIf()
...
phase_changed << 0
if EUDIf()(new_phase > phase):
    phase_changed << 1
EUDEndIf()
if EUDIf()(new_phase < phase):
    phase_changed << 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct variable initialization
boss_hp = session.var(BOSS_MAX_HP, name="boss_hp")
phase = session.var(1, name="phase")

# Direct phase transition gates
if (phase == 1) and (boss_hp <= 2000):
    phase = 2
    phase_timer = 0
    CreateUnit(4, "Zerg Hydralisk", LOC_SPAWN, Player8)
    MinimapPing(LOC_SPAWN)

if (phase == 2) and (boss_hp <= 1000):
    phase = 3
    phase_timer = 0
    CreateUnit(6, "Zerg Zergling", LOC_SPAWN, Player8)
    MinimapPing(LOC_SPAWN)
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 23 T | 35 T | **17 T** | **-26% vs Legacy, -51% vs Proto (v2 Wins)** |
| **Payload Size** | ~8,900 bytes | ~13,500 bytes | **6,208 bytes** | **-30% Payload Footprint** |
| **VProc Allocation** | 7 VProcs (504 B) | Dynamic | **0 VProcs** | **100% ByteBuf lowering** |
| **Initialization** | Runtime `phase==0` branch | Constructor init | Native `session.var(...)` | Zero startup triggers |

---

## 4. Architectural Mechanics

### Elimination of Scratch Transition State
- Legacy eudplib code requires multiple scratch variables (`new_phase`, `phase_changed`) and multi-stage branch chains (`new_phase > phase`, `new_phase < phase`) to emulate edge-triggered phase shifts without `!=`.
- Modern eudkit expresses state machine transitions directly as composite conditions `(phase == 1) and (boss_hp <= 2000)`, emitting optimal single-trigger transition checks.

### Zero-Startup-Cost Variable Lowering
- Legacy variables default to 0, forcing maps to run runtime initialization triggers on loop 1.
- In eudkit v2, initial values are written directly into map memory at compile time, eliminating initialization triggers entirely.
