/**
 *
 */

import {
    Array,
    EPD,
    dwread,
    dwread_epd,
    dwwrite,
    dwwrite_epd,
    every,
    session,
} from "eudkit";

export const PLAYERS = 4;
export const SETTLE_PERIOD = 120;
export const INTEREST_FLOOR = 1000;
export const INTEREST_DIVISOR = 20;
export const INTEREST_CAP = 200;
export const EXCHANGE_GAS_MIN = 500;
export const EXCHANGE_GAS = 100;
export const EXCHANGE_ORE = 150;
export const ORE_BASE = 0x57F0F0;
export const GAS_BASE = 0x57F120;
export const interest_paid = new Array(PLAYERS);
export let exchanges_total = session.var(0, "exchanges_total");
export let no_interest_ticks = session.var(0, "no_interest_ticks");
export let bank_ticks = session.var(0, "bank_ticks");

export function main(): void {
    every(SETTLE_PERIOD, () => {
        bank_ticks += 1;
        for (let p = 0; p < PLAYERS; p++) {
            let ore_addr = (ORE_BASE + (4 * p));
            let gas_addr = (GAS_BASE + (4 * p));
            let ore_now = dwread(ore_addr);
            let gas_now = dwread(gas_addr);
            if ((ore_now >= INTEREST_FLOOR)) {
                let interest = ((ore_now / INTEREST_DIVISOR) | 0);
                if ((interest > INTEREST_CAP)) {
                    interest = INTEREST_CAP;
                }
                ore_now += interest;
                dwwrite(ore_addr, ore_now);
                let _tmp1 = dwread_epd(EPD(0x6FE000 + 0) + p);
                dwwrite_epd(EPD(0x6FE000 + 0) + p, (_tmp1 + interest));
            } else {
                no_interest_ticks += 1;
            }
            if ((gas_now >= EXCHANGE_GAS_MIN)) {
                dwwrite(gas_addr, (gas_now - EXCHANGE_GAS));
                dwwrite(ore_addr, (ore_now + EXCHANGE_ORE));
                exchanges_total += 1;
            }
        }
    });
}
