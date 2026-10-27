# Scenario 32: Random Lottery and Prize Tier Distribution — Architecture Comparison

## 1. Scenario Intent
Every 200 trigger loops, draws a pseudo-random number (0..99) for each of 4 active players:
- **Tiers**:
  - `roll < 5`: 500 Ore (Jackpot tier, increments `jackpots`).
  - `roll < 25`: 100 Ore (Big tier).
  - `roll < 60`: 25 Ore (Small tier).
  - `roll >= 60`: No prize.
- Tracks total draws made, total ore disbursed, and lowest roll seen (`best_roll`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Relies on legacy f_dwrand EUDFunc call sites and manual zero-init guard
lottery_timer += 1
if EUDIf()(lottery_timer >= LOTTERY_PERIOD):
    lottery_timer << 0
    for player in (Player1, Player2, Player3, Player4):
        roll << f_dwrand() % 100
        draws += 1
        if EUDIf()(roll < JACKPOT_ROLL):
            DoActions(SetResources(player, Add, JACKPOT_ORE, Ore))
            total_paid += JACKPOT_ORE
            jackpots += 1
        ...
        # Legacy variable zero-init workaround
        if EUDIf()(seen_any == 0):
            best_roll << roll
            seen_any << 1
        EUDEndIf()
        if EUDIf()(roll < best_roll):
            best_roll << roll
        EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Inlined LCG PRNG, declarative with every(200):, and direct variable initialization
with every(LOTTERY_PERIOD):
    for p in range(PLAYERS_COUNT):
        seed = seed * 1664525 + 1013904223
        roll = seed % 100
        draws += 1

        if roll < JACKPOT_ROLL:
            SetResources(p, Add, JACKPOT_ORE, Ore)
            total_paid += JACKPOT_ORE
            jackpots += 1
        elif roll < BIG_ROLL:
            SetResources(p, Add, BIG_ORE, Ore)
            total_paid += BIG_ORE
        elif roll < SMALL_ROLL:
            SetResources(p, Add, SMALL_ORE, Ore)
            total_paid += SMALL_ORE

        # best_roll initialized to 100; eliminates seen_any flag
        if roll < best_roll:
            best_roll = roll
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~450 T (with f_dwrand) | 490 T | **300 T** | **-33% vs Legacy, -39% vs Proto (v2 Wins)** |
| **Payload Size** | ~130,000 bytes | ~145,000 bytes | **70,048 bytes** | **-46% Payload Footprint** |
| **PRNG Engine** | `f_dwrand` EUDFunc calls | Ad-hoc RNG | Inlined 32-bit LCG | Zero function call overhead |
| **Edge Guard** | `seen_any` flag & branch | Runtime flag | Constant init (`100`) | Eliminates branch & flag variable |

---

## 4. Architectural Mechanics

### Inlined PRNG Arithmetic Lowering
- Legacy eudplib requires calling `f_dwrand()` via an EUDFunc trampoline for each player roll. This emits argument registers, return registers, and function call overhead 4 times every cycle.
- Modern eudkit inlines the Linear Congruential Generator step (`seed = seed * A + C`) directly into integer arithmetic operations, generating tightly scheduled 32-bit math triggers.

### Non-Zero Variable Initialization
- Modern eudkit supports compile-time initialization values for variables (`session.var(100)`). This allows `best_roll` to initialize above the maximum roll (99), removing legacy `seen_any` branch triggers entirely.
