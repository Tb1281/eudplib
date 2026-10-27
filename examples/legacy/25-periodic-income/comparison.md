# Scenario 25: Periodic Income and Milestone Economy — Architecture Comparison

## 1. Scenario Intent
Distributes periodic economic revenue to Players 1 through 4:
- Every 150 trigger loops, grants 50 Ore and 20 Gas per player.
- Every 4th payout, adds a 100-Ore bonus per player.
- Accumulates lifetime resource metrics (`total_ore_paid`, `total_gas_paid`).

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
income_timer += 1
if EUDIf()(income_timer >= INCOME_PERIOD):
    income_timer << 0
    payouts += 1
    milestone_counter += 1

    is_milestone << 0
    if EUDIf()(milestone_counter >= MILESTONE_EVERY):
        milestone_counter << 0
        milestones += 1
        is_milestone << 1
    EUDEndIf()

    for player in (Player1, Player2, Player3, Player4):
        DoActions([
            SetResources(player, Add, ORE_PER_PAYOUT, Ore),
            SetResources(player, Add, GAS_PER_PAYOUT, Gas),
        ])
        if EUDIf()(is_milestone == 1):
            DoActions(SetResources(player, Add, MILESTONE_ORE, Ore))
        EUDEndIf()
    ...
```

### Modern eudkit v2 (`v2.py`)
```python
with every(INCOME_PERIOD):
    payouts += 1
    milestone_counter += 1
    is_milestone = 0

    if milestone_counter >= MILESTONE_EVERY:
        milestone_counter = 0
        milestones += 1
        is_milestone = 1

    for p in range(PLAYERS_COUNT):
        SetResources(p, Add, ORE_PER_PAYOUT, Ore)
        SetResources(p, Add, GAS_PER_PAYOUT, Gas)
        if is_milestone == 1:
            SetResources(p, Add, MILESTONE_ORE, Ore)
    ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 33 T | 32 T | **16 T** | **-51.5% Triggers (v2 Wins)** |
| **Payload Size** | ~14,000 bytes | ~13,500 bytes | **5,232 bytes** | **-62% Payload** |
| **VProc Allocation** | 7 VProcs (504 B) | Dynamic | **0 VProcs** | **100% ByteBuf lowering** |
| **Timer Model** | Manual `income_timer` var | `every(150)` | `with every(150):` | Zero boilerplate |

---

## 4. Architectural Mechanics

### Action Batching & Loop Inlining
- Legacy wraps resource actions in repetitive `DoActions` calls, adding macro jumps.
- Modern `eudkit` unrolls the 4-player resource grants at compile time and packs the `SetResources` actions into minimal trigger slots. Concurrently, all state variables (`payouts`, `milestone_counter`, `milestones`, `total_ore_paid`) are lowered to 4-byte `ByteBuf` storage, requiring **0 VProcs** and yielding **16 triggers**.

### Declarative Timing
- `with every(150):` eliminates error-prone manual timer increment/reset boilerplate, lowering directly to a countdown memory trigger.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
