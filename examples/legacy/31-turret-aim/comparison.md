# Scenario 31: Fixed Turret Aiming and Range Tracking — Architecture Comparison

## 1. Scenario Intent
A stationary turret at (1280, 1280) tracks a moving target coordinates (`target_x`, `target_y`):
- Calculates coordinate offsets: `dx = |target_x - 1280|`, `dy = |target_y - 1280|`.
- Computes Euclidean distance to target.
- If within range (`dist <= 384`) and cooldown is 0:
  - Fires at target: kills 1 enemy Zergling at location 6 for Player 8 (`KillUnitAt`).
  - Resets firing cooldown to 24 frames.
- If out of range: increments `out_of_range`.
- Tracks longest distance observed.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Invokes expensive iterative square-root EUDFunc every single frame
dist << f_sqrt(dx * dx + dy * dy)

if EUDIf()([dist > longest, dist <= MAX_TRACK_DIST]):
    longest << dist
EUDEndIf()

if EUDIf()(cooldown >= 1):
    cooldown -= 1
EUDEndIf()

if EUDIf()(dist <= RANGE):
    if EUDIf()(cooldown == 0):
        DoActions(KillUnitAt(1, UNIT_ZERGLING, LOC_AIM_ZONE, Player8))
        shots += 1
        cooldown << FIRE_COOLDOWN
    EUDEndIf()
if EUDElse()():
    out_of_range += 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Ideal Intent: Squared distance comparison eliminates square-root approximation
dist_sq = dx * dx + dy * dy

if (dist_sq > longest_sq) and (dist_sq <= MAX_TRACK_DIST_SQ):
    longest_sq = dist_sq

if cooldown >= 1:
    cooldown -= 1

if dist_sq <= RANGE_SQ:
    if cooldown == 0:
        KillUnitAt(1, "Zerg Zergling", LOC_AIM_ZONE, Player8)
        shots += 1
        cooldown = FIRE_COOLDOWN
else:
    out_of_range += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~360 T (with f_sqrt) | 410 T | **280 T** | **-22% vs Legacy, -32% vs Proto (v2 Wins)** |
| **Payload Size** | ~105,000 bytes | ~120,000 bytes | **78,864 bytes** | **-25% Payload Footprint** |
| **Distance Lowering** | `f_sqrt` binary search | Runtime math | Exact `dist_sq <= RANGE^2` | Avoids 16-step root search |
| **Action Lowering** | Raw trigger `KillUnitAt` | Unchecked action | First-class `KillUnitAt` | Strongly typed trigger action |

---

## 4. Architectural Mechanics

### Ideal Intent: Squared Range Comparison
- Legacy code invokes `f_sqrt` (a 16-step binary search EUDFunc) every frame just to compare against constant range 384. This is an anti-pattern in high-performance trigger code.
- Modern eudkit embraces ideal game engine design: comparing `dx * dx + dy * dy <= 147,456` gives exact geometric equivalence without computing a single square root.

### First-Class Tactical Actions
- `KillUnitAt` is integrated directly as a typed `StockAction` in the backend compiler, emitting optimal action opcode 23 without intermediate script wrappers.
