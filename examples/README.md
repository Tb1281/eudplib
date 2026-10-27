# eudkit Examples: Legacy v1 vs Modern v2

This directory contains standalone, runnable examples contrasting **legacy `eudplib` (v1)** patterns with **modern `eudkit` (v2)** architecture across the core domains of StarCraft EUD map programming.

Each directory contains:
- `legacy_v1.py`: How triggers were constructed in legacy `eudplib` v1.
- `eudkit_v2.py`: The equivalent program rewritten using modern `eudkit` v2.
- `README.md`: Deep architectural analysis comparing memory models, compiler passes, and trigger payloads.

---

## Example Directory Index

| Directory | Domain | Legacy `eudplib` (v1) | Modern `eudkit` (v2) | Architecture Benefit |
| :--- | :--- | :--- | :--- | :--- |
| [`01_variables_and_math/`](file:///home/dong/eudplib/examples/01_variables_and_math/README.md) | Variables & Math | `EUDVariable`, `<<`, temporary trigger ladders | `session.var(...)`, native arithmetic, constant folding | **-80% payload**: 4-byte cell storage vs 72-byte VProcs |
| [`02_control_flow/`](file:///home/dong/eudplib/examples/02_control_flow/README.md) | Control Flow | `EUDIf()()`, `EUDElse()()`, `EUDWhile()()`, `EUDBreak()` | Native Python `if/elif/else`, `while`, `break`, `continue` | **-91% payload**: Action coalescing (up to 64/block) |
| [`03_functions_and_inlining/`](file:///home/dong/eudplib/examples/03_functions_and_inlining/README.md) | Functions & Inlining | `@ep.EUDFunc`, double-copy parameter passing | `@session.fn(inline=True)`, direct-return convention | **-91% payload**: Zero-overhead AST inlining |
| [`04_memory_and_cunit/`](file:///home/dong/eudplib/examples/04_memory_and_cunit/README.md) | Memory & CUnit | Raw pointer arithmetic (`0x59CCA8`), repetitive bit ladders | Typed `CUnit(0)`, named fields, `ReadTable` sharing | **-62% payload**: Deduplicated bit-read triggers |
| [`05_strings_and_display/`](file:///home/dong/eudplib/examples/05_strings_and_display/README.md) | Strings & Display | `f_eprintln`, runtime XOR decryption, 150KB+ string bloat | `session.map_string()`, residue-stride CPString (`mod 12`) | **-97% payload**: Static STRx relocation & direct streaming |
| [`06_reactive_rules/`](file:///home/dong/eudplib/examples/06_reactive_rules/README.md) | Reactive Rules | Manual tick decrements (`timer -= 1`), fragile `Wait()` | Declarative `with once():`, `with every(24):`, `with when(...)` | **Multiplayer safety**: Zero desyncs, clean event model |

---

## Language Parity: Python & TypeScript

`eudkit` v2 provides first-class language parity between Python and TypeScript. The reactive primitives and builder APIs adapt naturally to each language's native idioms:

### Python: Context Manager Syntax
```python
import eudkit
from eudkit import once, every, when, DisplayText, SetResources, Player1, Add, Ore

session = eudkit.Session("input.scx")

@session.fn
def game_loop():
    # Execute once on startup
    with once():
        DisplayText("Welcome to StarCraft EUD!")

    # Execute cyclically every 24 ticks (1 second on Fastest)
    with every(24):
        SetResources(Player1, Add, 50, Ore)
```

### TypeScript: Arrow Function Callback Syntax
```typescript
import { once, every, when, DisplayText, SetResources, Player1, Add, Ore } from "@eudkit/sdk";

// Execute once on startup
once(() => {
    DisplayText("Welcome to StarCraft EUD!");
});

// Execute cyclically every 24 ticks (1 second on Fastest)
every(24, () => {
    SetResources(Player1, Add, 50, Ore);
});
```

*(Note: In Python, callback syntax `once(lambda: ...)` and `every(24, lambda: ...)` is also supported interchangeably).*

---

## Running the Examples

Each modern v2 example can be run directly using `uv`:

```bash
# Example 01: Variables & Arithmetic
uv run python examples/01_variables_and_math/eudkit_v2.py

# Example 02: Control Flow
uv run python examples/02_control_flow/eudkit_v2.py

# Example 03: Functions & Inlining
uv run python examples/03_functions_and_inlining/eudkit_v2.py

# Example 04: Memory & CUnit Entities
uv run python examples/04_memory_and_cunit/eudkit_v2.py

# Example 05: Strings & Display
uv run python examples/05_strings_and_display/eudkit_v2.py

# Example 06: Reactive Rules & Events
uv run python examples/06_reactive_rules/eudkit_v2.py
```

Each run compiles the workload against `tests/outputmap/basemap/basemap_strx.scx` and outputs a compiled map `output_v2.scx` along with trigger counts and payload statistics.
