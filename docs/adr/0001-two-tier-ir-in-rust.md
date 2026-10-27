# Two-Tier Intermediate Representation (HIR and LIR) in Rust

`eudkit` decouples language Frontends (Python, TypeScript) from the StarCraft: Remastered compilation Backend via a pure-Rust two-tier Intermediate Representation (`HIR` and `LIR`), replacing legacy `eudplib`'s direct runtime construction of Python `RawTrigger` and `EUDObject` graphs.

In legacy `eudplib`, executing user code immediately instantiated Python trigger objects on a global stack with hardcoded byte offsets (such as `+ 8 + 320` into 2408-byte triggers). That design prevented whole-program optimization passes (constant folding, dead-code elimination, action coalescing across statement boundaries) and caused heavy Python↔Rust FFI ping-pong during dependency collection and allocation. By lowering Frontend intents into structured `HIR` first and `LIR` with symbolic `FieldRef` pointers second, the Rust Backend can optimize both compile time and emitted trigger count before resolving concrete byte/EPD addresses.
