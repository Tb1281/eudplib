# Scenario 29: Kill Bounty and Broadcast Announcement — Architecture Comparison

## 1. Scenario Intent
Monitors Player 1 Marine deaths to calculate kill bounties and format live server announcements:
- Computes kill delta `new_kills = cur_kills - prev_kills`.
- Calculates payout: `new_kills * 15`. If `new_kills >= 5`, awards flat +50 jackpot bonus.
- Disburses ore into Player 1's bank (`0x57F0F0`).
- Composes dynamic string `"Player 1 scored %d kills! Bounty: %d ore"` and displays it on screen.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Requires legacy DBString and f_dbstr_print with runtime XOR decryption
announce_buf = DBString(128)
...
cur_kills << f_dwread_epd(EPD(MARINE_DEATHS_P1))
if EUDIf()(cur_kills > prev_kills):
    new_kills << cur_kills - prev_kills
    bounty << new_kills * BOUNTY_PER_KILL
    if EUDIf()(new_kills >= JACKPOT_MIN_KILLS):
        bounty += JACKPOT_BONUS
        jackpots += 1
    EUDEndIf()
    DoActions(SetDeaths(EPD(ORE_ADDR_P1), Add, bounty, 0))
    f_dbstr_print(announce_buf, "Player 1 scored ", new_kills,
                  " kills! Bounty: ", bounty, " ore")
    announce_buf.Display()
    announcements += 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Consolidated StringBuffer with format strings
announce_buf = session.string(128)
...
cur_kills = dwread_epd(EPD(MARINE_DEATHS_P1))
if cur_kills > prev_kills:
    new_kills = cur_kills - prev_kills
    bounty = new_kills * BOUNTY_PER_KILL
    if new_kills >= JACKPOT_MIN_KILLS:
        bounty += JACKPOT_BONUS
        jackpots += 1
    SetDeaths(EPD(ORE_ADDR_P1), Add, bounty, 0)
    announce_buf.printf("Player 1 scored %d kills! Bounty: %d ore", new_kills, bounty)
    announce_buf.display()
    announcements += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~175 T | 215 T | **142 T** | **-19% vs Legacy, -34% vs Proto (v2 Wins)** |
| **Payload Size** | ~52,000 bytes | ~64,000 bytes | **36,856 bytes** | **-29% Footprint** |
| **String Architecture** | Legacy `DBString` XOR | Ad-hoc buffers | Single `StringBuffer` | Residue-stride CPString lowering |
| **Formatting API** | Multi-arg `f_dbstr_print` | String concats | Typed `printf("%d...")` | Standard C-style format strings |

---

## 4. Architectural Mechanics

### Single Dynamic String Architecture
- Following user architectural direction (Ticket #27), ambiguous string APIs (`EUDString`, `DBString`) were removed in favor of `session.string()`.
- Modern `StringBuffer` compiles to static STRx storage relocated at `0x191943C8`, emitting residue-stride CP writes without runtime decryption overhead.

### Amortized ReadTable Engine
- Materialization of the player deaths counter utilizes the backend's unified 80-trigger ReadTable, sharing bit-extraction mechanics across memory access operations.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
