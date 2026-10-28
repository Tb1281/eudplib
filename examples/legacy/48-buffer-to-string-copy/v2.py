# 48 - runtime buffer -> string table copy (modern eudkit)
#
# Expresses ideal intent:
# 1. Single dynamic string architecture: consolidated on modern StringBuffer (ADR 0004).
# 2. Replaces fragile runtime CP block copy (`f_repmovsd_epd`) with static STRx
#    relocation and residue-stride CPString updates.
# 3. Clean local session state.
import eudkit


from eudkit import session

status_buf = session.string(64)

wave = session.var(0)
kills = session.var(0)
gold = session.var(0)
wave_timer = session.var(0)

filled_wave = session.var(-1)
filled_kills = session.var(-1)
filled_gold = session.var(-1)
filled_timer = session.var(-1)
copies_done = session.var(0)
line_version = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global filled_wave, filled_kills, filled_gold, filled_timer
    global copies_done, line_version

    stale = 0
    if filled_wave != wave:
        stale = 1
    if filled_kills != kills:
        stale = 1
    if filled_gold != gold:
        stale = 1
    if filled_timer != wave_timer:
        stale = 1

    if stale == 1:
        status_buf.printf("Wave %d | Kills %d | Gold %d | Timer %d", wave, kills, gold, wave_timer)
        status_buf.display_text()
        filled_wave = wave
        filled_kills = kills
        filled_gold = gold
        filled_timer = wave_timer
        copies_done += 1
        line_version += 1
