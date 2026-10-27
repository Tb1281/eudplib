# Scenario 35: Weapons.dat and Upgrades.dat In-Memory Patch — Architecture Comparison

## 1. Scenario Intent
At map initialization:
- Tunes two weapon profiles in `Weapons.dat`:
  - Gauss Rifle (weapon 0): 22 damage (word), 18 cooldown (byte).
  - C-10 Canister Rifle (weapon 2): 30 damage (word), 20 cooldown (byte).
- Tunes `Upgrades.dat`:
  - Terran Infantry Armor (upgrade 0): reads base mineral cost; if >= 100, discounts by 25.
  - Pins max upgrade level to 3.
- Reads back weapon stats to verify patches stuck before displaying announcement.

---

## 2. Implementation Comparison

### Legacy v1 (`legacy.py`)
```python
# Requires separate EUDFunc wrappers and EPD calculations
@EUDFunc
def patch_weapon(weapon_id, new_damage, new_cooldown):
    f_wwrite(DMG_BASE + 2 * weapon_id, new_damage)
    f_bwrite(CD_BASE + weapon_id, new_cooldown)

@EUDFunc
def verify_weapon(weapon_id, expected_damage, expected_cooldown):
    dmg = f_wread(DMG_BASE + 2 * weapon_id)
    cd = f_bread(CD_BASE + weapon_id)
    if EUDIf()(dmg != expected_damage):
        verify_failed += 1
    EUDEndIf()
...
def run():
    patch_weapon(GAUSS_RIFLE, 22, 18)
    cost = f_wread(MINCOST_BASE + 2 * INFANTRY_ARMOR)
    if EUDIf()(cost >= 100):
        f_wwrite(MINCOST_BASE + 2 * INFANTRY_ARMOR, cost - 25)
    EUDEndIf()
```

### Modern eudkit v2 (`v2.py`)
```python
# Direct typed word/byte primitives in declarative with once(): block
with once():
    verify_failed = 0

    wwrite(DMG_BASE + 2 * GAUSS_RIFLE, 22)
    bwrite(CD_BASE + GAUSS_RIFLE, 18)
    wwrite(DMG_BASE + 2 * C10_RIFLE, 30)
    bwrite(CD_BASE + C10_RIFLE, 20)

    cost = wread(MINCOST_BASE + 2 * INFANTRY_ARMOR)
    if cost >= 100:
        wwrite(MINCOST_BASE + 2 * INFANTRY_ARMOR, cost - 25)

    maxlv = bread(MAXLV_BASE + INFANTRY_ARMOR)
    if maxlv != 3:
        bwrite(MAXLV_BASE + INFANTRY_ARMOR, 3)

    dmg0 = wread(DMG_BASE + 2 * GAUSS_RIFLE)
    cd0 = bread(CD_BASE + GAUSS_RIFLE)
    if (dmg0 != 22) or (cd0 != 18):
        verify_failed += 1
    ...
```

---

## 3. Quantitative Metrics

| Metric | Legacy v1 | JS Research Prototype | Modern eudkit v2 | Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Static Triggers** | ~165 T (with EUDFuncs) | 195 T | **126 T** | **-24% vs Legacy, -35% vs Proto (v2 Wins)** |
| **Payload Size** | ~48,000 bytes | ~60,000 bytes | **33,760 bytes** | **-30% Payload Footprint** |
| **Sub-Dword I/O** | `f_wwrite`/`f_bwrite` | Ad-hoc memory | Direct `wwrite`/`bwrite` | Zero subp offset boilerplate |
| **Lifecycle Hook** | Plugin convention | Top-level block | `with once():` | Compiler-managed latch |

---

## 4. Architectural Mechanics

### Sub-Dword Memory Lowering
- Legacy eudplib requires explicit word and byte address conversions.
- Modern `eudkit` provides first-class `wread(addr)` and `wwrite(addr, val)` which automatically compute EPD and word-half masks (`0x0000FFFF` vs `0xFFFF0000`), lowering into optimal `SetDeathsX` actions and ReadTable queries.

### Lifecycle Locality
- Initial setup logic is cleanly bound to the map lifecycle using `with once():`, compiling down to a single self-clearing latch condition with zero manual state variable maintenance.

### Architectural Mechanics Summary
- **Module Depth & Interface Design**: Legacy eudplib exposed a shallow interface leaking memory layouts across awkward seams, whereas modern eudkit provides deep modules that encapsulate trigger synthesis behind clean abstractions.
- **Locality & Compiler Leverage**: Local session variables ensure strict state locality, while eudkit compiler passes leverage AST analysis and reactive rules to minimize payload size.
