# Migrating from legacy eudplib (v1) & epScript to eudkit (v2)

> [!WARNING]
> **Deprecation Notice (v1 Retirement)**
> Legacy `eudplib` (v1) and `epScript` are deprecated and retired.
> Importing `eudplib` now emits a `FutureWarning` directing developers to migrate to `eudkit`.
> All active development, performance optimizations, and tooling support occur exclusively in `eudkit` (v2).
> Please use this guide and the automated `eudkit migrate` CLI to migrate existing projects.

This guide documents how to migrate StarCraft: Remastered map projects from legacy `eudplib` (v1) and `epScript` to modern `eudkit` (v2), featuring automated project conversion via `eudkit migrate` and side-by-side pattern comparisons for both TypeScript (`@eudkit/sdk`) and Python v2.

---

## 1. Overview & Architectural Shift

Legacy `eudplib` relied on global mutable Python state, immediate trigger instantiation without whole-program optimization, manual variable type selection (`EUDVariable`, `EUDLightVariable`, `EUDXVariable`), and a C++ Lemon parser (`epScript`) that lacked IDE autocomplete and static type validation.

**`eudkit` (v2)** modernizes the toolchain:

| Architectural Concern | Legacy `eudplib` (v1) / `epScript` | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **State Management** | Module-level Python globals, phase managers | Isolated `Compilation Session` & read-only `Map Context` |
| **IR & Optimizations** | Direct trigger emission in Python with hardcoded offsets | Two-tier pure-Rust IR (`HIR` $\to$ `LIR`) with whole-program optimization |
| **Variable Allocations** | Manual choice of `EUDVariable` / `EUDLightVariable` | Unified `var_()` with automatic Rust **Storage Lowering** (4B cell vs. 72B `VProc`) |
| **Language Tooling** | C++ Lemon parser (`libepScriptLib`), no LSP or autocomplete | First-class TypeScript (`@eudkit/sdk`, oxc AST compiler) and Python v2 DSL (`@session.fn`) |
| **Function Calls** | Double-copy calling convention, runtime return triggers | Automatic HIR inlining + single-action direct-return calling convention |
| **Binary Relocation** | Dynamic 1.16.1 heap relocations (`vector_reloc` / `payload_reloc`) | Compile-time static `STRx` relocation (`0x191943C8`) |
| **Map Protection** | Unconditional runtime XOR decrypt loops & `MRGN` scrambling | Opt-in, deterministic Backend pass (`obscurate = true`, `obscurate_seed`) |
| **Project Configuration** | Legacy INI files (`.edd` / `.eds`) | Modern declarative TOML (`eudkit.toml`) |

---

## 2. One-Command Migration (`eudkit migrate`)

`eudkit` provides an automated CLI subcommand to migrate legacy `.edd`/`.eds` project configurations and `.eps` epScript source files into idiomatic TypeScript and `eudkit.toml`.

### Migrating an entire `.edd` or `.eds` project

```bash
# Migrates project.edd, converts referenced .eps files, and outputs eudkit.toml
eudkit migrate project.edd

# Compile the migrated project into a valid .scx map
eudkit build --config eudkit.toml
```

### Migrating a standalone `.eps` file

```bash
# Migrates main.eps directly to TypeScript main.ts
eudkit migrate src/main.eps -o src/main.ts
```

---

## 3. Side-by-Side Comparisons

### 3.1 Project Configuration (`.edd` $\to$ `eudkit.toml`)

Legacy map projects used INI files parsed by `euddraft`:

#### Legacy `.edd` / `.eds` (v1)
```ini
[main]
input: maps/basemap.scx
output: build/output.scx

[eudTurbo]
[freezeFilter]

[src/main.eps]
```

#### Modern `eudkit.toml` (v2)
```toml
[project]
name = "my_map"
entry = "src/main.ts"
dts = "types/map.d.ts"

[build]
input_map = "maps/basemap.scx"
output_map = "build/output.scx"
obscurate = false
preserve_classic_triggers = true
```

---

### 3.2 Variable Declarations & Automatic Storage Lowering

In legacy `eudplib`, mapmakers had to manually choose variable types to balance trigger count and functionality:
- `EUDVariable`: 72-byte `VProc` trigger slot (allows dynamic copying via self-modifying code, but heavy).
- `EUDLightVariable`: 4-byte memory cell (saves triggers, but cannot be copied dynamically).
- `EUDXVariable`: Bitmasked variable.

In `eudkit`, you declare a unified variable (`var_` in TypeScript or `session.var` in Python). The Rust Backend runs an **automatic Storage Lowering pass** that inspects variable liveness and usage: variables only read or modified with constants are stored in 4-byte memory cells with zero trigger overhead; variables requiring dynamic copy become single-slot 72-byte `VProcs`.

#### Legacy `epScript` (v1)
```javascript
var counter = 0;
static var staticFlag = 1;
const minerals = 500;
```

#### Modern TypeScript (`@eudkit/sdk`) (v2)
```typescript
import { var_ } from "@eudkit/sdk";

const counter = var_(0);
const staticFlag = var_(1);
const minerals = 500;
```

#### Modern Python v2 (`eudkit`)
```python
counter = session.var(0, name="counter")
static_flag = session.var(1, name="static_flag")
minerals = 500
```

---

### 3.3 Control Flow

Legacy Python v1 required cumbersome `if EUDIf()(): ... EUDElse()(): ... EUDEndIf()` blocks, while epScript used C-style control structures lowered directly to unoptimized triggers.

`eudkit` allows standard language control structures (`if / else`, `while`, `for`, `switch`) in both TypeScript and Python v2.

#### Legacy `eudplib` Python (v1)
```python
if EUDIf()(counter.AtLeast(15)):
    minerals.AddNumber(100)
    SetResources(Player1, Add, 100, Ore)
if EUDElse()():
    minerals.SetNumber(0)
EUDEndIf()

while EUDWhile()(counter.AtLeast(1)):
    counter.SubtractNumber(1)
EUDEndWhile()
```

#### Modern TypeScript (`@eudkit/sdk`) (v2)
```typescript
import { var_, SetResources, Player1, Add, Ore } from "@eudkit/sdk";

if (counter.atLeast(15)) {
    minerals.add(100);
    SetResources(Player1, Add, 100, Ore);
} else {
    minerals.setTo(0);
}

while (counter.atLeast(1)) {
    counter.subtract(1);
}

// Compile-time unrolling
for (let i = 0; i < 4; i++) {
    SetDeaths(Player1, Add, 10, 0);
}
```

#### Modern Python v2 (`eudkit`)
```python
@session.fn
def game_loop():
    if counter >= 15:
        minerals += 100
        session.set_resources(Player1, Modifier.Add, 100, Resource.Ore)
    else:
        minerals.set_to(0)

    while counter >= 1:
        counter -= 1
```

---

### 3.4 Functions & Calling Conventions

Legacy functions (`@EUDFunc`, `@EUDTypedFunc`, or epScript `function`) used a caller/callee double-copy convention: parameters were copied into argument registers, and returned values copied into temporary variables, creating 4+ triggers per call.

In `eudkit`:
1. Small functions are **automatically inlined** at HIR level with constant folding, eliminating 100% of function call trigger overhead.
2. Non-inlined functions use a **single-action direct-return calling convention** where return values are written directly to target locations without double copying.

#### Legacy `epScript` (v1)
```javascript
function addBonus(score, multiplier) {
    var total = score * multiplier;
    return total;
}

function main() {
    var res = addBonus(10, 2);
}
```

#### Modern TypeScript (`@eudkit/sdk`) (v2)
```typescript
import { var_, Var, PreserveTrigger } from "@eudkit/sdk";

export function addBonus(score: Var, multiplier: Var): Var {
    const total = var_(score * multiplier);
    return total;
}

export function main() {
    const res = var_(0);
    res.setTo(addBonus(10, 2));
    PreserveTrigger();
}
```

---

### 3.5 Memory Access & Game Structs (`CUnit`)

In legacy code, `CUnit` access frequently required raw EPD arithmetic and offset calculations (`u.owner`, `dwwrite_epd`). In `eudkit`, game structs are typed with properties mapped to engine offsets.

#### Legacy `epScript` (v1)
```javascript
function updateUnit(ptr, epd) {
    const u = CUnit(epd, ptr=ptr);
    if (u.owner == $P8) {
        u.hitPoints = 10000;
        u.posX += 10;
    }
    dwwrite_epd(epd + 0x50/4, 42);
}
```

#### Modern TypeScript (`@eudkit/sdk`) (v2)
```typescript
import { CUnit, Player8, SetMemory, SetTo } from "@eudkit/sdk";

export function updateUnit(ptr: number, epd: number) {
    const u = CUnit.cast(epd, ptr);
    if (u.player == Player8) {
        u.hitPoints = 10000;
        u.posX += 10;
    }
    SetMemory(epd + 0x50 / 4, SetTo, 42);
}
```

---

### 3.6 String Printing & Display

Legacy string formatting used `f_eprintln` / `DBString` with non-deterministic runtime XOR decryption loops that inflated trigger counts.

`eudkit` uses compile-time static `STRx` relocation with high-performance `CPString` writes (`mod 12` unit-stride residue batches), producing deterministic, zero-overhead text display.

#### Legacy `eudplib` (v1)
```python
from eudplib import f_eprintln

f_eprintln("Player {} score: {}", current_player, score)
```

#### Modern TypeScript (`@eudkit/sdk`) (v2)
```typescript
import { DisplayText, DisplayTextId, MapString } from "@eudkit/sdk";

DisplayText("Welcome to the game!");
```

---

### 3.7 Map Protection & Obscuration (`obscurate`)

Legacy `eudplib` unconditionally ran XOR decryption loops on strings and randomized object tables even during debug development.

`eudkit` separates map protection into an explicit, deterministic, seedable Backend pass controlled in `eudkit.toml`:

```toml
[build]
# Default is false: 0 obfuscation triggers, deterministic static strings
obscurate = false

# When releasing, enable protection with a fixed seed for reproducible builds
# obscurate = true
# obscurate_seed = 0xDEADBEEFCAFE
```

When enabled, `eudkit` protects output maps with:
1. Seeded LIR object allocation order shuffling before `stack_objects`.
2. Unit-offset bias (`12 * u` compensation) on EPD accesses.
3. String encryption with runtime XOR decode stubs.
4. `MRGN` section coordinate scrambling and debug string stripping.

---

## 4. Migration Checklist

- [ ] Run `eudkit migrate <project.edd>` to convert project config to `eudkit.toml` and epScript files to `.ts`.
- [ ] Run `eudkit inspect <map.scx> --dts types/map.d.ts` to generate typed autocomplete definitions for your map's locations, units, switches, and strings.
- [ ] Verify imported scripts compile cleanly with `eudkit dump-ir --config eudkit.toml`.
- [ ] Build the final `.scx` map with `eudkit build --config eudkit.toml`.
