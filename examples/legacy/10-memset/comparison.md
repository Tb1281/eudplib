# Scenario 10: Memset (Buffer Clear) — Architecture Comparison

## 1. Scenario Intent
Clears a designated 64-dword scratch buffer to 0 whenever a request flag (`clear_flag != 0`) is set, resetting the flag and incrementing a counter on completion.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
scratch = Db(b"\x00" * (64 * 4))
scratch_epd = EPD(scratch)
...
if EUDIf()(clear_flag != 0):
    idx << 0
    if EUDWhile()(idx < 64):
        f_dwwrite_epd(scratch_epd + idx, 0)
        idx << idx + 1
    EUDEndWhile()

    clear_flag << 0
    clears_done << clears_done + 1
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
if clear_flag != 0:
    idx = 0
    while idx < 64:
        dwwrite_epd(SCRATCH_EPD + idx, 0)
        idx += 1

    clear_flag = 0
    clears_done += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 13 T | 11 T | **0.85x** | **-15% Triggers (v2 Wins)** |
| **Payload Size** | ~5,200 bytes | 4,368 bytes | **0.84x** | **-16% Binary Size** |
| **VProc Allocation** | 3 VProcs (216 B) | 2 VProcs (144 B) | **0.67x** | -33% VProcs |
| **Control Flow Overhead** | `EUDIf() + EUDWhile()` | Native `while` + `if` | — | Zero macro overhead |

---

## 4. Architectural Mechanics

### Compiler Optimization & Address Patch Inlining
- **Legacy v1**: Emits generic function call prologue and epilogue for each iteration in `f_dwwrite_epd`, computing target EPD additions across separate actions.
- **Modern v2**: The `eudkit` LIR compiler fuses constant addition `SCRATCH_EPD + idx` directly into the destination field of the write action, saving 2 triggers in the loop kernel.

### Depth & Ergonomics
- The implementation in modern v2 is direct, structured, and free of macro boilerplate, providing an intuitive interface for systems needing low-level buffer initialization.
