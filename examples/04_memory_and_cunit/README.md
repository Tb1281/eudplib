# Example 04: Memory & Game Structs

Comparison of direct memory manipulation, entity wrappers, and array storage between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **Entity Interface** | Raw EPD math (`cunit_epd + 2`, `cunit_epd + 10`) | Typed `CUnit` wrapper with named properties (`unit.hp`, `unit.pos`) |
| **Bit-Read Generation** | Each `f_dwread_epd` call generated a separate 32-bit trigger ladder | **ReadTable Deduplication**: All reads share 1 global 32-bit trigger ladder |
| **Array Types** | `EUDArray` requiring manual pointer math | `Array` (static) and `VArray` (dynamic) with standard slice/index operators |
| **Trigger Overhead** | High (>300 triggers for reading 4 unit fields) | Low (~170 triggers including all struct, array, and BitTable logic) |

## Key Insights

### 1. ReadTable Bit Deduplication
In StarCraft EUD, reading a dynamic 32-bit dword requires testing powers of two ($2^0, 2^1, \dots, 2^{31}$).
In v1, calling `f_dwread_epd` multiple times generated redundant trigger ladders across the payload.
`eudkit` aggregates all memory reads across the entire compilation session into a centralized `ReadTable`. The 32 bit-reading triggers are instantiated exactly once and shared by all callers via pointer redirection.

### 2. High-Leverage Typed Structs
Rather than calculating manual byte offsets (`cunit_epd + 0x4C // 4`), developers interact with `unit.hp`, `unit.posX`, `unit.posY`, `unit.order`, and helpers like `unit.set_invincible()`.

## Running the Examples

```bash
eudkit build --input tests/outputmap/basemap/basemap_strx.scx --output output_v2.scx --entry examples/04_memory_and_cunit/eudkit_v2.py
```
