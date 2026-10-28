/**
 *
 */

import {
    every,
    once,
    session,
} from "eudkit";

export let code_buf = session.string(64);
export let level = session.var(0, "level");
export let stage = session.var(0, "stage");
export let gold_tier = session.var(0, "gold_tier");
export let boss_flags = session.var(0, "boss_flags");
export let next_flag_bit = session.var(0, "next_flag_bit");
export let code_body = session.var(0, "code_body");
export let check = session.var(0, "check");
export let save_code = session.var(0, "save_code");
export let prints = session.var(0, "prints");
export let codes_shown = session.var(0, "codes_shown");

export function main(): void {
    once(() => {
        next_flag_bit = 1;
    });
    every(120, () => {
        stage += 1;
        if ((stage >= 10)) {
            stage = 0;
            if ((level < 20)) {
                level += 1;
            }
        }
        if ((((stage % 3) == 0) && (stage >= 1))) {
            boss_flags = (boss_flags | next_flag_bit);
            next_flag_bit = (next_flag_bit * 2);
        }
        code_body = ((((((level * 10) + stage) * 10) + gold_tier) * 1000) + boss_flags);
        check = (((((level * 3) + (stage * 5)) + (gold_tier * 7)) + (boss_flags * 11)) % 10);
        save_code = ((code_body * 10) + check);
        code_buf.printf("SAVE CODE: ", save_code);
        code_buf.display();
        prints += 1;
        codes_shown += 1;
    });
}
