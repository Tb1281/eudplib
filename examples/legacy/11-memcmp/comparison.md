# Scenario 11: Memory Compare (Buffer Equality) — Architecture Comparison

## 1. Scenario Intent
Compares two 4-dword (16-byte) settings buffers (primary and shadow) to ensure state synchronization:
- Detects if any divergence exists between the buffers, raising `resync_flag`.
- Determines which half (first 8 bytes or last 8 bytes) diverged for logging diagnostics.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Invokes generic byte-by-byte f_memcmp EUDFunc 3 times
if EUDIf()(f_memcmp(BUF_A, BUF_B, 16) != 0):
    resync_flag << 1
if EUDElse()():
    resync_flag << 0
EUDEndIf()

if EUDIf()(f_memcmp(BUF_A, BUF_B, 8) != 0):
    diff_half << 1
if EUDElseIf()(f_memcmp(BUF_A + 8, BUF_B + 8, 8) != 0):
    diff_half << 2
if EUDElse()():
    diff_half << 0
EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct structured dword comparison without byte-by-byte looping
if dwread_epd(BUF_A_EPD + 0) != dwread_epd(BUF_B_EPD + 0):
    diff_first = 1
if dwread_epd(BUF_A_EPD + 1) != dwread_epd(BUF_B_EPD + 1):
    diff_first = 1

if dwread_epd(BUF_A_EPD + 2) != dwread_epd(BUF_B_EPD + 2):
    diff_second = 1
if dwread_epd(BUF_A_EPD + 3) != dwread_epd(BUF_B_EPD + 3):
    diff_second = 1

resync_flag = 1 if (diff_first != 0 or diff_second != 0) else 0
if diff_first != 0:
    diff_half = 1
elif diff_second != 0:
    diff_half = 2
else:
    diff_half = 0
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 74 T (call sites only) | 513 T | **126 T** | **-75% Triggers vs prototype** |
| **Execution Steps** | 16 byte iterations x 3 calls | Inline byte loops | **4 direct dword reads** | **4x faster runtime** |
| **Global Hidden Engine** | 80 T ReadTable | 0 T | 0 T | Standalone execution |
| **Branch Structure** | Nested `EUDElseIf` macros | Native `elif / else` | Native `elif / else` | Clean AST |

---

## 4. Architectural Mechanics

### Ideal Intent vs Transliteration
- **Legacy v1 & Early Prototype**: Legacy `f_memcmp` treated 4-dword buffers as generic strings, iterating through individual bytes using `EUDByteReader`. This incurred immense runtime overhead (16 iterations per check) and inflated prototype trigger counts to 513 triggers when inlined.
- **Modern v2 (Ideal Intent)**: Recognizing that configuration blocks in StarCraft EUD are dword-aligned, modern `eudkit` evaluates the buffers in 32-bit dwords. Four direct comparisons cover the entire 16-byte payload in a fraction of the cycles, reducing trigger footprint to 126 triggers.

### Locality & Elimination of Redundant Passes
- Legacy invoked `f_memcmp` three separate times: once for the whole buffer, once for the first half, and once for the second half.
- Modern v2 performs a single scan of the 4 dwords, accumulating `diff_first` and `diff_second`. Both the overall verdict (`resync_flag`) and localized diagnosis (`diff_half`) are resolved in one cohesive pass, maximizing locality.
