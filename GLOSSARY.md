# eudkit

A compiler, intermediate representation, and multi-language SDK for programming StarCraft: Remastered maps via Extended Unit Deaths (EUD) triggers and binary payload injection.

## Language

**Frontend**:
A host-language authoring layer (such as the Python DSL or TypeScript DSL) that captures a mapmaker's intent and emits an IR Module.
_Avoid_: Script, transpiler, wrapper

**IR (Intermediate Representation)**:
The language-agnostic program and map-mutation representation linking any Frontend to the Backend, consisting of **HIR** (structured control flow, variables, and game intents) and **LIR** (low-level triggers, memory objects, and relocation symbols).
_Avoid_: AST, bytecode, payload

**HIR (High-Level IR)**:
The structured representation of functions, virtual variables, control flow, and game intents before lowering to StarCraft trigger structures.
_Avoid_: AST, high-level triggers

**LIR (Low-Level IR)**:
The graph of low-level triggers, memory objects, and symbolic relocations ready for optimization, object-stacking allocation, and binary emission.
_Avoid_: Raw triggers, object graph

**Backend**:
The Rust compilation engine that lowers IR into StarCraft trigger graphs and EUD memory layouts, optimizes trigger count, resolves addresses, and packs the final Map.
_Avoid_: Allocator, bundler, runtime

**Map Context**:
The read-only schema of assets and entities (locations, strings, unit settings, players) extracted from an input `.scx`/`.scm` file for Frontend type-checking and symbol resolution.
_Avoid_: Map data, CHK globals, string map

**Compilation Session**:
An isolated, state-owning compilation instance that holds the `Map Context`, the `IR` under construction, and all configuration, replacing module-level global singletons.
_Avoid_: Global state, phase manager

**Symbolic Field Reference (`FieldRef`)**:
A typed LIR pointer to a specific field inside a trigger, condition, or action slot that resolves to a concrete byte or EPD offset only during final allocation.
_Avoid_: Trigger offset, magic offset

**Payload**:
The packed, relocated binary buffer of compiled triggers and EUD objects injected into the output map's `STRx` section.
_Avoid_: Byte dump, object buffer

**VProc (Variable Procedure)**:
A low-level trigger whose `SetDeaths` action holds a runtime variable in its value field and dynamically patches its destination EPD and `NextPtr` to copy that value into other memory locations.
_Avoid_: Variable trigger, EUDVarBuffer slot

**CurrentPlayer Cursor (`CP`)**:
StarCraft's global current-player memory register (`0x6509B0`, accessed via `Player 13`), used by the Backend as a relative base pointer to read or write sequential dwords with minimal triggers.
_Avoid_: CP trick, curpl global

**Obscuration (`obscurate`)**:
An opt-in, seedable Backend transformation pass that protects the emitted `Payload` and `CHK` sections against decompilers (via object-layout shuffling, unit-offset randomization, string encryption, and `MRGN`/section scrambling) without altering runtime semantics.
_Avoid_: Randomization, hardcoded obfuscation
