# Scenario 24: Wave Spawner System — Architecture Comparison

## 1. Scenario Intent
Implements a 10-wave single-player tower defense spawner:
- Every 240 frames, initiates the next wave with dynamic quota `4 + 2 * wave`.
- Streams one unit per frame alternating between Zergling (location 1) and Hydralisk (location 2) for Player 8.
- Tracks completed waves and pays out a 150-Ore milestone bonus every 5 completed waves.
- Flags victory upon clearing wave 10.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
if EUDIf()(victory == 0):
    spawn_timer += 1
    if EUDIf()([spawn_timer >= WAVE_PERIOD, wave_active == 0, wave < MAX_WAVE]):
        wave += 1
        quota << 4 + 2 * wave
        spawned_in_wave << 0
        wave_active << 1
        spawn_timer << 0
    EUDEndIf()
    if EUDIf()(wave_active == 1):
        if EUDIf()(spawn_alt == 0):
            DoActions(CreateUnit(1, UNIT_ZERGLING, LOC_SPAWN_A, Player8))
            spawn_alt << 1
        if EUDElse()():
            DoActions(CreateUnit(1, UNIT_HYDRALISK, LOC_SPAWN_B, Player8))
            spawn_alt << 0
        EUDEndIf()
        ...
```

### Modern eudkit v2 (`v2.py`)
```python
if victory == 0:
    spawn_timer += 1
    if (spawn_timer >= WAVE_PERIOD) and (wave_active == 0) and (wave < MAX_WAVE):
        wave += 1
        quota = 4 + 2 * wave
        spawned_in_wave = 0
        wave_active = 1
        spawn_timer = 0

    if wave_active == 1:
        if spawn_alt == 0:
            CreateUnit(1, "Zerg Zergling", 1, Player8)
            spawn_alt = 1
        else:
            CreateUnit(1, "Zerg Hydralisk", 2, Player8)
            spawn_alt = 0
        ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 40 T | 46 T (var-vs-var workaround) | **21 T** | **-47.5% Triggers (v2 Wins)** |
| **Payload Size** | ~17,000 bytes | ~20,000 bytes | **11,384 bytes** | **-33% Payload** |
| **VProc Allocation** | 10 VProcs (720 B) | Dynamic | **3 VProcs (216 B)** | **-70% VProcs** |
| **Branch Flattening** | Nested macro blocks | Workaround flags | Native logical `and` | Optimal IR |

---

## 4. Architectural Mechanics

### Condition Conjunction Fusion
- Legacy v1 uses an explicit array of conditions `[spawn_timer >= WAVE_PERIOD, wave_active == 0, wave < MAX_WAVE]` wrapped in macro `EUDIf()`.
- Modern `eudkit` evaluates compound Python logical expressions (`and`) and flattens them into a single StarCraft trigger condition set. It solves the early prototype's variable-versus-variable comparison bug and generates only **21 triggers**, cutting legacy trigger consumption in half.

### Storage Lowering
- Counter variables (`spawned_in_wave`, `total_spawned`, `waves_completed`, `bonus_counter`) are lowered to compact 4-byte ByteBuf cells with zero runtime VProc overhead.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
