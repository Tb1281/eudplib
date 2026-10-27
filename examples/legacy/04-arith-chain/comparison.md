# Scenario 04: Arithmetic Expression Chain — Architecture Comparison

## 1. Scenario Intent
Computes multi-stage composite arithmetic expressions across game variables on every trigger frame:
- Combat attack calculation involving addition, multiplication, subtraction, integer division, and modulus:
  `((atk_base + atk_bonus) * atk_mul - armor_flat) // divisor % wrap_mod`
- Timer bitwise extraction: high word shifting (`>> 16`), mask (`& 0xFFFF`), scaling, and phase derivation (`// 60`, `% 60`, `^`).
- Bounded iterative loop: unrolled 24-step linear summation.
- Value boundary clamping.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Broken up into multiple manual steps and function trampolines
result << ((atk_base + atk_bonus) * atk_mul - armor_flat)
result << f_div(result, divisor)[0]
result << f_div(result, wrap_mod)[1]

hi << (raw_timer >> 16)
cd_ticks << f_div(hi * 7 + (raw_timer & 0xFFFF), 3600)[1]
q60, r60 = f_div(cd_ticks, 60)
cd_phase << ((q60 & 0xFF) ^ r60)

loop_guard << 0
if EUDWhile()(loop_guard < 24):
    result << result + loop_guard * 3
    loop_guard << loop_guard + 1
EUDEndWhile()
```

### Modern eudkit v2 (`v2.py`)
```python
# Single unified AST compound expressions with native // and %
result = ((atk_base + atk_bonus) * atk_mul - armor_flat) // divisor % wrap_mod

hi = raw_timer >> 16
cd_ticks = (hi * 7 + (raw_timer & 0xFFFF)) % 3600
cd_phase = ((cd_ticks // 60) & 0xFF) ^ (cd_ticks % 60)

loop_guard = 0
while loop_guard < 24:
    result += loop_guard * 3
    loop_guard += 1
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | Modern eudkit v2 | Ratio | Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | 480 T (call sites only) | 673 T (inlined sequences) | **1.40x** | Inline expansion vs Trampoline calls |
| **f_div Trampoline Calls** | 4 separate calls | 0 calls (inlined IR) | — | Zero function call overhead |
| **Runtime Execution Speed** | Slower (function jumps) | Faster (pipelined VProcs) | — | Direct sequential execution |
| **Expression Form** | Manual 4-line breakdown | 1-line native Python | — | High leverage, zero boilerplate |

---

## 4. Architectural Mechanics

### Module Depth & Expression Inlining
- **Legacy v1 (Fragmented Interface)**: Legacy `eudplib` could not compile compound division or modulo expressions in a single line. Developers were forced to break expressions across multiple statements, manually invoking `f_div(...)` and indexing the resulting tuple `[0]` or `[1]`. Each call introduced an `EUDFunc` trampoline frame with return pointer rewriting.
- **Modern v2 (Deep Expression Lowering)**: `eudkit` treats integer division (`//`) and modulo (`%`) as first-class HIR arithmetic nodes. The Rust compiler analyzes the expression tree and directly lowers it into optimized inline `SeqCompute` VProc chains. There are no function boundaries, no stack frames, and no tuple unpacking overhead.

### Locality & Zero-Cost Intermediate Values
- In legacy v1, intermediate results had to be assigned back to `result` or temporary variables repeatedly (`result << f_div(result, ...)[0]`).
- In modern v2, the compiler builds an expression graph, allocating temporary scratch storage only when necessary and reusing scratch cells across unrelated terms in the expression.
