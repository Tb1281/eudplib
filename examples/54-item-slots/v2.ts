/**
 *
 */

import {
    Array,
    Clear,
    EPD,
    Set,
    SetSwitch,
    Switch,
    dwread_epd,
    dwwrite_epd,
    every,
    session,
} from "eudkit";

export const EMPTY = 0;
export const POTION = 1;
export const ELIXIR = 2;
export const SLOTS = 4;
export const HERO_MAX_HP = 1200;
export const HERO_MAX_MANA = 100;
export const POTION_HEAL = 300;
export const POTION_PERIOD = 90;
export const ELIXIR_PERIOD = 270;
export const DAMAGE_PERIOD = 30;
export const AMBIENT_DAMAGE = 7;
export const SW_USE_POTION = 4;
export const SW_USE_ELIXIR = 5;
export const SW_DROP = 6;
export const slots = new Array(SLOTS);
export let hero_hp = session.var(900, "hero_hp");
export let hero_mana = session.var(40, "hero_mana");
export let potions_used = session.var(0, "potions_used");
export let elixirs_used = session.var(0, "elixirs_used");
export let drops = session.var(0, "drops");
export let failed_uses = session.var(0, "failed_uses");
export let potions_found = session.var(0, "potions_found");
export let elixirs_found = session.var(0, "elixirs_found");

export function main(): void {
    every(DAMAGE_PERIOD, () => {
        if ((hero_hp >= (AMBIENT_DAMAGE + 1))) {
            hero_hp -= AMBIENT_DAMAGE;
        }
    });
    every(POTION_PERIOD, () => {
        let found_slot = (-1);
        for (let i = 0; i < 4; i++) {
            let _tmp1 = dwread_epd(EPD(0x6FE000 + 0) + i);
            if ((_tmp1 == EMPTY)) {
                if ((found_slot == (-1))) {
                    found_slot = i;
                }
            }
        }
        if ((found_slot != (-1))) {
            dwwrite_epd(EPD(0x6FE000 + 0) + found_slot, POTION);
            potions_found += 1;
        }
    });
    every(ELIXIR_PERIOD, () => {
        found_slot = (-1);
        for (let i = 0; i < 4; i++) {
            let _tmp2 = dwread_epd(EPD(0x6FE000 + 0) + i);
            if ((_tmp2 == EMPTY)) {
                if ((found_slot == (-1))) {
                    found_slot = i;
                }
            }
        }
        if ((found_slot != (-1))) {
            dwwrite_epd(EPD(0x6FE000 + 0) + found_slot, ELIXIR);
            elixirs_found += 1;
        }
    });
    if (Switch(SW_USE_POTION, Set)) {
        SetSwitch(SW_USE_POTION, Clear);
        found_slot = (-1);
        for (let i = 0; i < 4; i++) {
            let _tmp3 = dwread_epd(EPD(0x6FE000 + 0) + i);
            if ((_tmp3 == POTION)) {
                if ((found_slot == (-1))) {
                    found_slot = i;
                }
            }
        }
        if ((found_slot != (-1))) {
            if ((hero_hp <= (HERO_MAX_HP - POTION_HEAL))) {
                hero_hp += POTION_HEAL;
            } else {
                hero_hp = HERO_MAX_HP;
            }
            dwwrite_epd(EPD(0x6FE000 + 0) + found_slot, EMPTY);
            potions_used += 1;
        } else {
            failed_uses += 1;
        }
    }
    if (Switch(SW_USE_ELIXIR, Set)) {
        SetSwitch(SW_USE_ELIXIR, Clear);
        found_slot = (-1);
        for (let i = 0; i < 4; i++) {
            let _tmp4 = dwread_epd(EPD(0x6FE000 + 0) + i);
            if ((_tmp4 == ELIXIR)) {
                if ((found_slot == (-1))) {
                    found_slot = i;
                }
            }
        }
        if ((found_slot != (-1))) {
            hero_hp = HERO_MAX_HP;
            hero_mana = HERO_MAX_MANA;
            dwwrite_epd(EPD(0x6FE000 + 0) + found_slot, EMPTY);
            elixirs_used += 1;
        } else {
            failed_uses += 1;
        }
    }
    if (Switch(SW_DROP, Set)) {
        SetSwitch(SW_DROP, Clear);
        let drop_slot = (-1);
        for (let i = 3; i < (-1); i += (-1)) {
            let _tmp5 = dwread_epd(EPD(0x6FE000 + 0) + i);
            if ((_tmp5 != EMPTY)) {
                if ((drop_slot == (-1))) {
                    drop_slot = i;
                }
            }
        }
        if ((drop_slot != (-1))) {
            dwwrite_epd(EPD(0x6FE000 + 0) + drop_slot, EMPTY);
            drops += 1;
        } else {
            failed_uses += 1;
        }
    }
}
