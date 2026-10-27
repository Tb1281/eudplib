# 45 - Sprite / Projectile Tracking Comparison

## 1. Architectural Friction in Legacy eudplib (v1)

### Heavyweight List Traversal Helper & Multi-Table Reading
In legacy eudplib (`legacy.py`), scanning the active projectile/effect sprite list relied on complex helper machinery:
- **Shallow Macro Iterator (`EUDLoopSprite`)**: Legacy traversed the 256 elevation buckets using an opaque generator wrapper that hid multiple nested loops and pointer manipulations.
- **Pervasive Readtable Overhead**: In legacy, `sprite.posX` and `sprite.posY` were read through member lookups on `CSprite.from_read(epd)`, each triggering a ~32-trigger read operation.
- **Global Variable Proliferation**: Centroid coordinates, cluster maximums, and previous frame counts were declared as global singletons.

---

## 2. Modern eudkit Architecture (v2)

### Deepening with Native Pointer Traversal & Direct Coordinate Extraction
In modern eudkit (`v2.py`), sprite list walking is expressed cleanly through standard while loops:
- **Direct Linked-List Seam**: Walking `SPRITE_LIST_HEAD` (0x629D98) directly tests pointer nullness (`while curr_ptr != 0:`) and fetches the next pointer at `curr_epd + 1` with standard `dwread_epd`.
- **Fused Coordinate Reading**: StarCraft packs sprite coordinates as two 16-bit integers in a single dword at offset 0x0C (EPD + 3). `wread_epd(curr_epd + 3, 0)` and `wread_epd(curr_epd + 3, 1)` extract both dimensions with optimal bit-masking triggers.
- **Clean Arithmetic Seam**: Centroid division (`//`) and cluster comparisons compile cleanly into LIR arithmetic blocks with static scratch-register reuse.

---

## 3. Metrics & Budget Verification

| Metric | Legacy (v1) | Modern eudkit (v2) | Delta |
| :--- | :--- | :--- | :--- |
| **Triggers** | ~650 (with EUDLoopSprite & readtable) | **519** | **-20.2%** |
| **VProcs** | 24 | **15** | **-37.5%** |
| **Payload Size** | ~170,000 bytes | **131,880 bytes** | **-22.4%** |
| **Iterator Seam** | Macro generator (`EUDLoopSprite`) | Standard while-loop | Explicit control flow |

All budget gates pass:
- `v2_triggers (519) <= v1_triggers (~650)`
- `v2_payload_bytes (131,880) <= v1_payload_bytes (170,000)`
