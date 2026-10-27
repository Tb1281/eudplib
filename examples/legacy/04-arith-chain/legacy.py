"""04-arith-chain (legacy) — composite integer arithmetic over game vars.

Intent: r = ((a + b) * c - d) / e % f computed fresh every trigger
loop for a set of stat variables, plus a second chain mixing shifts,
masks and div/mod unpacking. Legacy idiom: operator overloads on
EUDVariable build SeqCompute chains for +,-,*,&,>>,^, but division
and modulo go through the f_div EUDFunc, whose (quotient, remainder)
pair the caller unpacks — each f_div call is a full function call
with return-address trampoline triggers.
"""

from eudkit import *

atk_base = EUDVariable(120)
atk_bonus = EUDVariable(35)
atk_mul = EUDVariable(3)
armor_flat = EUDVariable(17)
divisor = EUDVariable(4)
wrap_mod = EUDVariable(1000)
result = EUDVariable(0)

raw_timer = EUDVariable(0x12345678)
cd_ticks = EUDVariable(0)
cd_phase = EUDVariable(0)
loop_guard = EUDVariable(0)
hi = EUDVariable(0)


def run():
    global result, cd_ticks, cd_phase, loop_guard, hi
    # Chain 1: (((a + b) * c) - d) / e % f. The +,-,* part is one
    # operator chain; / and % each cost an f_div call.
    result << ((atk_base + atk_bonus) * atk_mul - armor_flat)
    result << f_div(result, divisor)[0]
    result << f_div(result, wrap_mod)[1]

    # Chain 2: isolate the high word, then a rotate-ish mix. One
    # f_div unpacks (quotient, remainder) for the two phase terms.
    hi << (raw_timer >> 16)
    cd_ticks << f_div(hi * 7 + (raw_timer & 0xFFFF), 3600)[1]
    q60, r60 = f_div(cd_ticks, 60)
    cd_phase << ((q60 & 0xFF) ^ r60)

    # Chain 3: bounded accumulation loop over a plain compare.
    loop_guard << 0
    if EUDWhile()(loop_guard < 24):
        result << result + loop_guard * 3
        loop_guard << loop_guard + 1
    EUDEndWhile()

    # Keep the computation live: reset once the wrapped value is high.
    if EUDIf()(result >= 500):
        result << 0
    EUDEndIf()
