"""LEGACY large-table linear search with early exit (benchmark 49).

Same intent as ours.js: a 512-entry item database (key and price
in parallel EUDArrays). When a search is requested, scan linearly
from index 0 and stop at the first key match (EUDBreakIf); record
the found index and price, or count a miss after a full scan.
Bookkeeping: searches run, total probes spent, cheapest price
ever found.

Every probe materializes the key with an array read -- the loop
compares values, so the read table does the work per element.
"""

from eudkit import *

TABLE_N = 512

item_key = EUDArray(TABLE_N)
item_price = EUDArray(TABLE_N)

search_key = EUDVariable(0)
search_request = EUDVariable(0)  # 1 = run a search this frame
found_index = EUDVariable(0xFFFFFFFF)
found_price = EUDVariable(0)
searches_done = EUDVariable(0)
not_found_count = EUDVariable(0)
total_probes = EUDVariable(0)
best_price = EUDVariable(0xFFFFFFFF)

idx = EUDVariable(0)
probes = EUDVariable(0)


def run():
    global search_request, found_index, found_price, searches_done
    global not_found_count, total_probes, best_price, idx, probes
    if EUDIf()(search_request == 1):
        idx << 0
        probes << 0
        found_index << 0xFFFFFFFF

        # Linear scan; the break check sits at the top of the body
        # so `probes` counts exactly the entries examined.
        if EUDWhile()(idx < TABLE_N):
            probes += 1
            if EUDIf()(item_key[idx] == search_key):
                found_index << idx
                found_price << item_price[idx]
                EUDBreak()
            EUDEndIf()
            idx += 1
        EUDEndWhile()

        if EUDIf()(found_index == 0xFFFFFFFF):
            not_found_count += 1
        EUDEndIf()
        if EUDIf()(found_index != 0xFFFFFFFF):
            if EUDIf()(found_price < best_price):
                best_price << found_price
            EUDEndIf()
        EUDEndIf()

        total_probes += probes
        searches_done += 1
        search_request << 0
    EUDEndIf()
