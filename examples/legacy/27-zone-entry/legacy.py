"""LEGACY zone entry detection (benchmark 27).

Same intent as ours.js: watch Player 1's Marines entering the
zone at location 2. Each ENTRY (transition from outside to inside)
pays a 25-ore bonus (first 10 entries only), pings the zone, and
counts the entry; exits are counted too, and loops spent inside
accumulate. The legacy idiom is a hand-maintained state variable:
the Bring condition is sampled every loop and compared against
the remembered `inside` flag to synthesize rising/falling edges.
"""

from eudkit import *

LOC_ZONE = 2             # location id (numeric: no basemap lookup)
UNIT_MARINE = 0          # unit id (numeric form, as in the pilot)
ENTRY_BONUS = 25
PAID_ENTRIES = 10        # only the first 10 entries pay out

inside = EUDVariable()       # remembered state: 1 while P1 is in the zone
entries = EUDVariable()      # zone entries so far
exits = EUDVariable()        # zone exits so far
paid_entries = EUDVariable()  # entries that received the bonus
bonus_ore_total = EUDVariable()  # lifetime bonus ore paid
loops_inside = EUDVariable()  # total loops spent inside the zone


def run():
    global inside, entries, exits, paid_entries, bonus_ore_total
    global loops_inside
    # Sample the zone condition once per loop. The absence sample
    # uses the engine-level negation: AtMost 0 == not AtLeast 1.
    present = Bring(Player1, AtLeast, 1, UNIT_MARINE, LOC_ZONE)
    absent = Bring(Player1, AtMost, 0, UNIT_MARINE, LOC_ZONE)

    # Rising edge: outside -> inside.
    if EUDIf()([present, inside == 0]):
        entries += 1
        if EUDIf()(paid_entries < PAID_ENTRIES):
            paid_entries += 1
            bonus_ore_total += ENTRY_BONUS
            DoActions(SetResources(Player1, Add, ENTRY_BONUS, Ore))
        EUDEndIf()
        DoActions(MinimapPing(LOC_ZONE))
        inside << 1
    EUDEndIf()

    # Falling edge: inside -> outside.
    if EUDIf()([inside == 1, absent]):
        exits += 1
        inside << 0
    EUDEndIf()

    # Dwell bookkeeping.
    if EUDIf()(inside == 1):
        loops_inside += 1
    EUDEndIf()
