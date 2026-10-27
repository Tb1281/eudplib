# Differential Corpus Benchmark Suite (v1 vs v2 Budget Gate Report)

Automated budget gate verification comparing legacy `eudplib` (v1) against `eudkit` (v2)
across representative real-world SCX workload kernels. Enforces strict gates:
- **Trigger Budget Gate**: `v2_triggers <= v1_triggers`
- **Payload Size Gate**: `v2_payload_bytes <= v1_payload_bytes`
- **Compile Time Gate**: `v2_compile_ms <= v1_compile_ms * 1.5`

## Summary Table

| Kernel | Workload Description | v1 Triggers | v2 Triggers | Δ Triggers | v1 Payload | v2 Payload | Δ Payload | v1 Time | v2 Time | Speedup | Budget Gate |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Kernel 1** | Compound Arithmetic & Bitwise Math | 344 | 62 | -82.0% | 120,675 B | 24,040 B | -80.1% | 260.2 ms | 41.2 ms | 6.32x | ✅ PASS |
| **Kernel 2** | Control Flow & Reactive Primitives | 499 | 28 | -94.4% | 154,807 B | 9,016 B | -94.2% | 255.3 ms | 18.7 ms | 13.68x | ✅ PASS |
| **Kernel 3** | Functions & Inlining | 385 | 42 | -89.1% | 132,751 B | 12,232 B | -90.8% | 252.5 ms | 18.8 ms | 13.41x | ✅ PASS |
| **Kernel 4** | Memory & Entities | 343 | 176 | -48.7% | 128,591 B | 47,592 B | -63.0% | 253.9 ms | 20.1 ms | 12.60x | ✅ PASS |
| **Kernel 5** | Strings & Formatting | 508 | 2 | -99.6% | 176,559 B | 4,488 B | -97.5% | 277.5 ms | 17.0 ms | 16.30x | ✅ PASS |
| **Kernel 6** | Full Map Payload Compilation | 559 | 134 | -76.0% | 193,343 B | 34,920 B | -81.9% | 270.0 ms | 19.9 ms | 13.59x | ✅ PASS |
| **Kernel 7** | TypeScript Frontend Lowering | 584 | 7 | -98.8% | 194,259 B | 5,944 B | -96.9% | 283.2 ms | 244.6 ms | 1.16x | ✅ PASS |
| **TOTAL** | **Aggregate Corpus** | **3,222** | **451** | **-86.0%** | **1,100,985 B** | **138,232 B** | **-87.4%** | **1852.6 ms** | **380.3 ms** | **4.87x** | **✅ PASS** |

## Architectural Optimization Insights

1. **Compound Arithmetic & Bitwise Math (Kernel 1)**:
   - Eudkit lowers constant-only variables into 4-byte `ByteBuf` storage with zero `vproc` overhead.
   - Constant expressions fold at compile time; dynamic arithmetic chains reuse scratch slots effectively.

2. **Control Flow & Reactive Primitives (Kernel 2)**:
   - Native `when`, `until`, `after`, `every`, and `once` avoid bulky timer state machines and fragile `Wait()` actions.
   - Action coalescing packs up to 64 actions per trigger block, reducing trigger count by >90%.

3. **Functions & Inlining (Kernel 3)**:
   - Small helper functions inline directly into caller ASTs with 0 call overhead.
   - Non-inlined functions leverage the direct-return convention with direct register assignment.

4. **Memory & Entities (Kernel 4)**:
   - `CUnit`, `CSprite`, and `Array`/`VArray` access shares bit-read triggers through the deduplicated `ReadTable`.
   - Radically shrinks trigger ladders compared to legacy repetitive `f_dwread_epd` expansions.

5. **Strings & Formatting (Kernel 5)**:
   - Residue-stride CPString streaming actions bypass v1's 150KB+ string decryption tables and runtime loops.
   - `MapString` deduplication reuses identical strings in session scope.

6. **Full Map Payload Compilation (Kernel 6)**:
   - End-to-end injection into `basemap_strx.scx` validates that the final `.scx` file size is ~40-80% smaller.

7. **TypeScript Frontend Lowering (Kernel 7)**:
   - `@eudkit/sdk` TypeScript source lowers through the Oxc-based AST pipeline into identical compact HIR and LIR.

## Verification Status

- **All Budget Gates Met**: `v2_triggers <= v1_triggers`, `v2_payload_bytes <= v1_payload_bytes`, `v2_compile_ms <= v1_compile_ms * 1.5`.
- **Deterministic Output**: Seeded and unseeded builds maintain exact byte-for-byte reproducibility.
- **Generated Timestamp**: 2026-10-27 12:47:45Z
