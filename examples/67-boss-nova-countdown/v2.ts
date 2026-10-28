/**
 *
 */

import {
    P1,
    PlayWAV,
    bread_epd,
    dwread,
    dwread_epd,
    dwwrite_epd,
    session,
    wread_epd,
} from "eudkit";

export const P1 = 0;
export const CYCLE = 600;
export const WARN_STEP = 24;
export const NOVA_DAMAGE = 25600;
export const ARENA_X0 = 1024;
export const ARENA_X1 = 3072;
export const ARENA_Y0 = 1024;
export const ARENA_Y1 = 3072;
export const FIRST_VISIBLE_UNIT = 0x628430;
export const DEATHS_BASE = 0x58A364;
export let cycle_t = session.var(0, "cycle_t");
export let warn_stage = session.var(3, "warn_stage");
export let nova_count = session.var(0, "nova_count");
export let warnings_shown = session.var(0, "warnings_shown");
export let last_blast_hits = session.var(0, "last_blast_hits");
export let total_hits = session.var(0, "total_hits");
export let uptr = session.var(0, "uptr");
export let uepd = session.var(0, "uepd");
export let scratch_hp = session.var(0, "scratch_hp");
export const msg_3 = session.map_string("Boss nova in 3...");
export const msg_2 = session.map_string("Boss nova in 2...");
export const msg_1 = session.map_string("Boss nova in 1...");

export function main(): void {
    cycle_t += 1;
    if (((warn_stage == 3) && (cycle_t >= (CYCLE - (3 * WARN_STEP))))) {
        PlayWAV("sound\glue\bnetclick.wav");
        msg_3.display();
        warn_stage = 2;
        warnings_shown += 1;
    }
    if (((warn_stage == 2) && (cycle_t >= (CYCLE - (2 * WARN_STEP))))) {
        PlayWAV("sound\glue\bnetclick.wav");
        msg_2.display();
        warn_stage = 1;
        warnings_shown += 1;
    }
    if (((warn_stage == 1) && (cycle_t >= (CYCLE - WARN_STEP)))) {
        PlayWAV("sound\glue\bnetclick.wav");
        msg_1.display();
        warn_stage = 0;
        warnings_shown += 1;
    }
    if ((cycle_t >= CYCLE)) {
        last_blast_hits = 0;
        uptr = dwread(FIRST_VISIBLE_UNIT);
        while ((uptr != 0)) {
            uepd = (((uptr - DEATHS_BASE) / 4) | 0);
            let _tmp1 = bread_epd((uepd + 19), 0);
            if ((_tmp1 == P1)) {
                let x = wread_epd((uepd + 10), 0);
                if (((x >= ARENA_X0) && (x <= ARENA_X1))) {
                    let y = wread_epd((uepd + 10), 1);
                    if (((y >= ARENA_Y0) && (y <= ARENA_Y1))) {
                        scratch_hp = dwread_epd((uepd + 2));
                        if ((scratch_hp > NOVA_DAMAGE)) {
                            dwwrite_epd((uepd + 2), (scratch_hp - NOVA_DAMAGE));
                        } else {
                            dwwrite_epd((uepd + 2), 0);
                        }
                        last_blast_hits += 1;
                        total_hits += 1;
                    }
                }
            }
            uptr = dwread_epd((uepd + 1));
        }
        nova_count += 1;
        cycle_t = 0;
        warn_stage = 3;
    }
}
