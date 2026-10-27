"""LEGACY runtime buffer -> string table copy (benchmark 48).

Same intent as ours.js: a 16-dword (64-byte) status line lives in a
private runtime buffer. Game logic updates four value dwords in it
(wave, kills, gold, timer); whenever any of them changes, the whole
buffer is copied into string-table slot 7's current data address --
the datadumper "copy" mode pattern, done at runtime: resolve the
slot's pointer with f_epdread_epd, then move the block with
f_repmovsd_epd (the CP-cursor mover). A classic trigger displays
the slot; the EUD side only keeps it fresh.

The buffer writes use f_dwwrite_epd because the destination is a
raw data-area EPD, not a variable.
"""

from eudkit import *
from eudkit.memio import f_dwwrite_epd, f_epdread_epd, f_repmovsd_epd

BUFFER_DWORDS = 16
STRING_TABLE_ADDRS = 0x191943C8
STATUS_STRID = 7

line_buf = Db(BUFFER_DWORDS * 4)
line_epd = EPD(line_buf)

# Value cells inside the buffer (dword offsets).
OFF_WAVE = 1
OFF_KILLS = 2
OFF_GOLD = 3
OFF_TIMER = 4

wave = EUDVariable(0)
kills = EUDVariable(0)
gold = EUDVariable(0)
wave_timer = EUDVariable(0)

filled_wave = EUDVariable(0xFFFFFFFF)
filled_kills = EUDVariable(0xFFFFFFFF)
filled_gold = EUDVariable(0xFFFFFFFF)
filled_timer = EUDVariable(0xFFFFFFFF)
copies_done = EUDVariable(0)
line_version = EUDVariable(0)


def run():
    global filled_wave, filled_kills, filled_gold, filled_timer
    global copies_done, line_version
    # Producer: any source value moved -> the line is stale.
    stale = EUDVariable(0)
    if EUDIf()(filled_wave != wave):
        stale << 1
    EUDEndIf()
    if EUDIf()(filled_kills != kills):
        stale << 1
    EUDEndIf()
    if EUDIf()(filled_gold != gold):
        stale << 1
    EUDEndIf()
    if EUDIf()(filled_timer != wave_timer):
        stale << 1
    EUDEndIf()

    if EUDIf()(stale == 1):
        # Refill the value cells, latch what was filled.
        f_dwwrite_epd(line_epd + OFF_WAVE, wave)
        f_dwwrite_epd(line_epd + OFF_KILLS, kills)
        f_dwwrite_epd(line_epd + OFF_GOLD, gold)
        f_dwwrite_epd(line_epd + OFF_TIMER, wave_timer)
        filled_wave << wave
        filled_kills << kills
        filled_gold << gold
        filled_timer << wave_timer

        # Resolve the string slot's current data address (the cell
        # holds a pointer; f_epdread_epd returns it as an EPD), then
        # move the whole 16-dword line in one block copy.
        dst_epd = f_epdread_epd(EPD(STRING_TABLE_ADDRS + 4 * STATUS_STRID))
        f_repmovsd_epd(dst_epd, line_epd, BUFFER_DWORDS)
        copies_done += 1
        line_version += 1
    EUDEndIf()
