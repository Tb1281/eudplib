# Example 05: Strings & Display

Comparison of string registration, display formatting, and text streaming between **legacy `eudplib` (v1)** and **modern `eudkit` (v2)**.

## Architectural Comparison

| Concept | Legacy `eudplib` (v1) | Modern `eudkit` (v2) |
| :--- | :--- | :--- |
| **String Storage** | Bloated runtime decryption tables (>150KB per map) | Static `STRx` relocation (`0x191943C8`) with deduplication |
| **Deduplication** | Partial / manual string caching | Automatic `MapString` session-scoped deduplication |
| **Text Display** | Runtime loop character copy (`DBString`) with XOR decoding | **Residue-stride CPString streaming**: grouped unit-stride CurrentPlayer writes |
| **Trigger Overhead** | >500 triggers for printing dynamic strings | **2 triggers** (clean CP pointer offset writes) |
| **Payload Reduction** | 177,387 bytes | 4,488 bytes (**-97.5% reduction**) |

## Key Insights

### 1. Residue-Stride CPString Streaming
In StarCraft, writing strings dynamically to the chat log or display buffer using triggers is traditionally done via `SetDeaths(CurrentPlayer, ...)`.
Legacy v1 compiled complex runtime loops that decrypted strings byte by byte, adding hundreds of triggers.
`eudkit` analyzes string residues modulo 12 (the 48-byte trigger action stride) and batches CurrentPlayer modifications into direct constant writes that execute in 1 or 2 triggers.

### 2. Static STRx Relocation
In SC:R (StarCraft: Remastered), STRx section strings are mapped into memory at a fixed address `0x191943C8`. `eudkit` precomputes all string offsets at compile time, completely eliminating 1.16.1 legacy heap search triggers.

### 3. Canonical `session.string()`
Dynamic formatted in-game strings are instantiated via the canonical `session.string(content)` factory (accepting either integer byte capacity or initial text). Format calls (`hud.printf("Score: %d", score)`) stream directly into the allocated STRx buffer without decryption loops.

## Running the Examples

```bash
eudkit build --input tests/outputmap/basemap/basemap_strx.scx --output output_v2.scx --entry examples/05_strings_and_display/eudkit_v2.py
```
