# 54 - item slots (legacy eudplib)
#
# Intent: a hero item belt with 4 slots. Slot values: 0 = empty,
# 1 = potion, 2 = elixir; the belt is a 4-dword EUDArray. Ground
# drops arrive on timers: a potion drops into the first empty slot
# every 90 loops, an elixir every 270 loops. Three request
# switches (set by hotkey/beacon triggers elsewhere) drive use:
#   SW 4: drink first potion  -> +300 HP (cap 1200)
#   SW 5: drink first elixir  -> HP and mana to full
#   SW 6: drop the LAST filled slot's item (lost)
# The hero takes 7 ambient damage every 30 loops so the belt has
# something to heal. Bookkeeping: potions/elixirs used, drops,
# failed uses (switch pressed with no matching item).
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production the body is the beforeTriggerExec hook; run() is
# that body verbatim. Switches are numeric ids (no basemap in the
# harness, exactly like the pilot scenarios). The scan helpers are
# plain Python codegen functions: each call emits its loop inline
# into run()'s scope, so every trigger lands inside the delta --
# the same accounting as the ours side's JS helper functions.
from eudkit import *

EMPTY = 0
POTION = 1
ELIXIR = 2
SLOTS = 4
HERO_MAX_HP = 1200
HERO_MAX_MANA = 100
POTION_HEAL = 300
POTION_PERIOD = 90      # loops between potion drops
ELIXIR_PERIOD = 270     # loops between elixir drops
DAMAGE_PERIOD = 30      # loops between ambient damage ticks
AMBIENT_DAMAGE = 7

SW_USE_POTION = 4       # switch ids
SW_USE_ELIXIR = 5
SW_DROP = 6

slots = EUDArray([0] * SLOTS)   # the item belt
hero_hp = EUDVariable(900)
hero_mana = EUDVariable(40)
potions_used = EUDVariable(0)
elixirs_used = EUDVariable(0)
drops = EUDVariable(0)          # items thrown away via SW_DROP
failed_uses = EUDVariable(0)    # use requests with no item
potions_found = EUDVariable(0)  # drops picked up
elixirs_found = EUDVariable(0)
scan_idx = EUDVariable(0)       # scratch: slot scan cursor
scan_found = EUDVariable(0)     # scratch: 1 when scan_idx hit
potion_tick = EUDVariable(0)    # hand-rolled period timers
elixir_tick = EUDVariable(0)
damage_tick = EUDVariable(0)


def scan_first(value):
    """Emit a scan for the first slot holding `value` (codegen).

    On exit scan_found says whether one exists; scan_idx is its
    index. Emitted inline at each call site.
    """
    global scan_idx, scan_found
    scan_idx << 0
    scan_found << 0
    if EUDWhile()(scan_idx < SLOTS):
        if EUDIf()(scan_found == 1):
            EUDBreak()
        EUDEndIf()
        if EUDIf()(slots[scan_idx] == value):
            scan_found << 1
        EUDEndIf()
        if EUDIf()(scan_found == 0):
            scan_idx += 1
        EUDEndIf()
    EUDEndWhile()


def scan_last_filled():
    """Emit a scan for the last non-empty slot (codegen)."""
    global scan_idx, scan_found
    scan_idx << SLOTS
    scan_found << 0
    if EUDWhile()(scan_idx >= 1):
        if EUDIf()(scan_found == 1):
            EUDBreak()
        EUDEndIf()
        scan_idx -= 1
        if EUDIf()(slots[scan_idx] != EMPTY):
            scan_found << 1
        EUDEndIf()
    EUDEndWhile()


def run():
    global hero_hp, hero_mana, potions_used, elixirs_used, drops
    global failed_uses, potions_found, elixirs_found
    global potion_tick, elixir_tick, damage_tick

    # Ambient damage: the reason the belt exists.
    damage_tick += 1
    if EUDIf()(damage_tick >= DAMAGE_PERIOD):
        damage_tick << 0
        if EUDIf()(hero_hp >= AMBIENT_DAMAGE + 1):
            hero_hp -= AMBIENT_DAMAGE
        EUDEndIf()
    EUDEndIf()

    # Ground drops into the first empty slot.
    potion_tick += 1
    if EUDIf()(potion_tick >= POTION_PERIOD):
        potion_tick << 0
        scan_first(EMPTY)
        if EUDIf()(scan_found == 1):
            slots[scan_idx] = POTION
            potions_found += 1
        EUDEndIf()
    EUDEndIf()
    elixir_tick += 1
    if EUDIf()(elixir_tick >= ELIXIR_PERIOD):
        elixir_tick << 0
        scan_first(EMPTY)
        if EUDIf()(scan_found == 1):
            slots[scan_idx] = ELIXIR
            elixirs_found += 1
        EUDEndIf()
    EUDEndIf()

    # Drink a potion.
    if EUDIf()(Switch(SW_USE_POTION, Set)):
        DoActions(SetSwitch(SW_USE_POTION, Clear))
        scan_first(POTION)
        if EUDIf()(scan_found == 1):
            if EUDIf()(hero_hp <= HERO_MAX_HP - POTION_HEAL):
                hero_hp += POTION_HEAL
            EUDEndIf()
            if EUDIf()(hero_hp > HERO_MAX_HP - POTION_HEAL):
                hero_hp << HERO_MAX_HP
            EUDEndIf()
            slots[scan_idx] = EMPTY
            potions_used += 1
        EUDEndIf()
        if EUDIf()(scan_found == 0):
            failed_uses += 1
        EUDEndIf()
    EUDEndIf()

    # Drink an elixir: full HP and mana.
    if EUDIf()(Switch(SW_USE_ELIXIR, Set)):
        DoActions(SetSwitch(SW_USE_ELIXIR, Clear))
        scan_first(ELIXIR)
        if EUDIf()(scan_found == 1):
            hero_hp << HERO_MAX_HP
            hero_mana << HERO_MAX_MANA
            slots[scan_idx] = EMPTY
            elixirs_used += 1
        EUDEndIf()
        if EUDIf()(scan_found == 0):
            failed_uses += 1
        EUDEndIf()
    EUDEndIf()

    # Throw away the last filled slot.
    if EUDIf()(Switch(SW_DROP, Set)):
        DoActions(SetSwitch(SW_DROP, Clear))
        scan_last_filled()
        if EUDIf()(scan_found == 1):
            slots[scan_idx] = EMPTY
            drops += 1
        EUDEndIf()
        if EUDIf()(scan_found == 0):
            failed_uses += 1
        EUDEndIf()
    EUDEndIf()
