# Scenario 30: Decimal Scoreboard Print — Architecture Comparison

## 1. Scenario Intent
Tracks Player 1 kill and death statistics across two memory-mapped deaths counters:
- Player 8 Zergling deaths = kills scored (`0x58A364 + 4*7 + 48*37`).
- Player 1 Marine deaths = deaths suffered (`0x58A364`).
- Periodically (every 120 trigger loops):
  - Score = max(0, kills * 100 - deaths * 30).
  - Accuracy = (kills * 100) // (kills + deaths) if kills + deaths > 0 else 0.
  - Formats and displays a dynamic decimal status line.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Requires manual f_div EUDFunc, scratch numerator/denominator, and DBString
board_buf = DBString(128)
...
print_timer += 1
if EUDIf()(print_timer >= PRINT_PERIOD):
    print_timer << 0
    if EUDIf()(kills_total + deaths_total >= 1):
        acc_num << kills_total * 100
        acc_den << kills_total + deaths_total
        accuracy << f_div(acc_num, acc_den)[0]
    if EUDElse()():
        accuracy << 0
    EUDEndIf()
    f_dbstr_print(board_buf, "SCORE: ", score, "  KILLS: ",
                  kills_total, "  DEATHS: ", deaths_total,
                  "  ACC: ", accuracy, "%")
    board_buf.Display()
    prints += 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Declarative reactive timing, native Python // division, and unified StringBuffer
board_buf = session.string(128)
...
with every(PRINT_PERIOD):
    total_events = kills_total + deaths_total
    if total_events >= 1:
        accuracy = (kills_total * 100) // total_events
    else:
        accuracy = 0

    board_buf.printf(
        "SCORE: %d  KILLS: %d  DEATHS: %d  ACC: %d%%",
        score,
        kills_total,
        deaths_total,
        accuracy,
    )
    board_buf.display()
    prints += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~480 T (with f_div) | 560 T | **416 T** | **-13% vs Legacy, -26% vs Proto (v2 Wins)** |
| **Payload Size** | ~135,000 bytes | ~160,000 bytes | **103,816 bytes** | **-23% Payload** |
| **Division Model** | `f_div` EUDFunc call site | Runtime divide | Native `//` operator | Direct 32-bit hardware division lowering |
| **Timer Model** | `print_timer` accumulator | Manual timer | `with every(120):` | Zero manual timer variables |

---

## 4. Architectural Mechanics

### Native Hardware Division Lowering
- Legacy eudplib lacks division operators on variables, requiring developers to invoke `f_div(acc_num, acc_den)[0]`, expanding into separate call sites and return variable unpackings.
- Modern `eudkit` lowers native Python integer division `//` directly into binary shift-and-subtract trigger ladders with zero function trampoline overhead.

### Consolidated StringBuffer and Reactive Timing
- Replaces legacy `DBString` with single modern `StringBuffer`.
- Cyclic interval execution is modeled with declarative `with every(PRINT_PERIOD):`, automatically handled by compiler timer registers.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
