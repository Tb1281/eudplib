"""LEGACY sprite / projectile tracking (benchmark 45).

Same intent as ours.js: every frame, walk the engine's sprite list
(projectile and effect sprites live here, not in the unit list).
Count the sprites inside the arena rectangle and keep their
centroid; when the count rises (new projectile/effect appeared),
record the new appearances and the largest cluster seen.

Legacy walks the 256 elevation buckets with EUDLoopSprite and reads
CSprite fields through member reads -- each read materializes the
value with the read table (~32T class for a dword), because the
loop hands you values, never conditions.
"""

from eudkit import *
from eudkit.eudlib.utilf.listloop import EUDLoopSprite
from eudkit.scdata import CSprite

ARENA_MIN = 512
ARENA_MAX = 3584

sprite_count = EUDVariable(0)
sum_x = EUDVariable(0)
sum_y = EUDVariable(0)
centroid_x = EUDVariable(0)
centroid_y = EUDVariable(0)
prev_count = EUDVariable(0)
appeared_total = EUDVariable(0)
largest_cluster = EUDVariable(0)


def run():
    global sprite_count, sum_x, sum_y, centroid_x, centroid_y
    global prev_count, appeared_total, largest_cluster
    sprite_count << 0
    sum_x << 0
    sum_y << 0

    for ptr, epd in EUDLoopSprite():
        sprite = CSprite.from_read(epd)
        x = sprite.posX
        y = sprite.posY
        if EUDIf()(x >= ARENA_MIN):
            if EUDIf()(x <= ARENA_MAX):
                if EUDIf()(y >= ARENA_MIN):
                    if EUDIf()(y <= ARENA_MAX):
                        sprite_count += 1
                        sum_x += x
                        sum_y += y
                    EUDEndIf()
                EUDEndIf()
            EUDEndIf()
        EUDEndIf()

    if EUDIf()(sprite_count > 0):
        centroid_x << sum_x // sprite_count
        centroid_y << sum_y // sprite_count
    EUDEndIf()

    # Hand-rolled rising edge on the count.
    if EUDIf()(sprite_count > prev_count):
        appeared_total += sprite_count - prev_count
        if EUDIf()(sprite_count > largest_cluster):
            largest_cluster << sprite_count
        EUDEndIf()
    EUDEndIf()
    prev_count << sprite_count
