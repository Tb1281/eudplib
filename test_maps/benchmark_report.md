# Differential Corpus Benchmark Suite (v1 vs v2 Budget Gate Report)

Automated budget gate verification comparing legacy `eudplib` (v1) against `eudkit` (v2)
across representative real-world SCX workload kernels. Enforces strict gates:
- **Trigger Budget Gate**: `v2_triggers <= v1_triggers`
- **Payload Size Gate**: `v2_payload_bytes <= v1_payload_bytes`
- **Compile Time Gate**: `v2_compile_ms <= v1_compile_ms * 1.5`

## Summary Table

| Kernel | Workload Description | v1 Triggers | v2 Triggers | Δ Triggers | v1 Payload | v2 Payload | Δ Payload | v1 Time | v2 Time | Speedup | Budget Gate |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Kernel 1** | Compound Arithmetic & Bitwise Math | 344 | 62 | -82.0% | 120,331 B | 24,040 B | -80.0% | 265.2 ms | 22.8 ms | 11.65x | ✅ PASS |
| **Kernel 2** | Control Flow & Reactive Primitives | 499 | 28 | -94.4% | 156,523 B | 9,016 B | -94.2% | 269.8 ms | 20.6 ms | 13.07x | ✅ PASS |
| **Kernel 3** | Functions & Inlining | 385 | 42 | -89.1% | 131,347 B | 12,232 B | -90.7% | 258.4 ms | 18.6 ms | 13.92x | ✅ PASS |
| **Kernel 4** | Memory & Entities | 343 | 176 | -48.7% | 127,399 B | 47,592 B | -62.6% | 265.5 ms | 20.4 ms | 13.00x | ✅ PASS |
| **Kernel 5** | Strings & Formatting | 508 | 2 | -99.6% | 176,151 B | 4,488 B | -97.5% | 291.8 ms | 16.9 ms | 17.32x | ✅ PASS |
| **Kernel 6** | Full Map Payload Compilation | 559 | 134 | -76.0% | 194,535 B | 34,920 B | -82.0% | 274.5 ms | 19.0 ms | 14.43x | ✅ PASS |
| **Kernel 7** | TypeScript Frontend Lowering | 584 | 7 | -98.8% | 194,935 B | 5,944 B | -97.0% | 288.3 ms | 238.2 ms | 1.21x | ✅ PASS |
| **TOTAL** | **Aggregate Corpus** | **3,222** | **451** | **-86.0%** | **1,101,221 B** | **138,232 B** | **-87.4%** | **1913.4 ms** | **356.4 ms** | **5.37x** | **✅ PASS** |

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
- **Generated Timestamp**: 2026-10-27 13:00:08Z
