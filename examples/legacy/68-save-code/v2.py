# 68 - save code encode + display (modern eudkit)
#
# Expresses ideal intent:
# 1. Consolidated `StringBuffer` API for single dynamic string architecture (ADR 0004).
# 2. Reactive rules (`with session.once():`, `with session.every(120):`) eliminating manual clocks.
# 3. Packed progression encoding and check digit generation with zero global singletons.
import eudkit


from eudkit import session

code_buf = session.string(64)

level = session.var(0)
stage = session.var(0)
gold_tier = session.var(0)
boss_flags = session.var(0)
next_flag_bit = session.var(0)
code_body = session.var(0)
check = session.var(0)
save_code = session.var(0)
prints = session.var(0)
codes_shown = session.var(0)

@session.fn(main_loop=True)
def game_loop() -> None:
    global level, stage, gold_tier, boss_flags, next_flag_bit
    global code_body, check, save_code, prints, codes_shown

    with session.once():
        next_flag_bit = 1

    with session.every(120):
        stage += 1
        if stage >= 10:
            stage = 0
            if level < 20:
                level += 1

        if stage % 3 == 0 and stage >= 1:
            boss_flags = boss_flags | next_flag_bit
            next_flag_bit = next_flag_bit * 2

        code_body = ((level * 10 + stage) * 10 + gold_tier) * 1000 + boss_flags
        check = (level * 3 + stage * 5 + gold_tier * 7 + boss_flags * 11) % 10
        save_code = code_body * 10 + check

        code_buf.printf("SAVE CODE: ", save_code)
        code_buf.display()
        prints += 1
        codes_shown += 1
