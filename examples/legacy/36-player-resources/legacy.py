# 36 - player resources: periodic bonus / tax (legacy eudplib)
#
# Intent: roughly every 5 seconds (84 trigger loops at Fastest), help
# every player who is low on minerals (+150 ore if below 500) and tax
# players floating too much gas (-50 gas if above 300). The resource
# counts live in per-player dword arrays in game memory:
#   minerals 0x57F0F0 + 4 * player, gas 0x57F120 + 4 * player
# (legacy scdata TrgPlayer arrays). Writes go through SetResources.
#
# Counting form (pilot convention): run() builds the scenario's
# triggers once; benchmarks/pilot/count_legacy.py calls it inside a
# fresh PushTriggerScope and reports the GetTriggerCounter() delta.
# In production this body is the beforeTriggerExec hook. Text is a Db
# string (no basemap in the harness, like the pilot scenarios). The
# f_dwread call sites ride the import-time readtable engine (80T,
# built once per session and outside the delta) -- the amortization
# asymmetry the pilot RESULTS calls out.
from eudkit import *

MINERAL_ADDR = 0x57F0F0
GAS_ADDR = 0x57F120
INTERVAL = 84          # ~5 s at Fastest (16.7 trigger loops / s)
LOW_MINERAL = 500
BONUS = 150
HIGH_GAS = 300
TAX = 50

tick = EUDVariable()
paid_count = EUDVariable()


@EUDFunc
def settle_player(player):
    global paid_count
    minerals = f_dwread(MINERAL_ADDR + 4 * player)
    if EUDIf()(minerals < LOW_MINERAL):
        DoActions(SetResources(player, Add, BONUS, Ore))
        paid_count += 1
    EUDEndIf()
    gas = f_dwread(GAS_ADDR + 4 * player)
    if EUDIf()(gas > HIGH_GAS):
        DoActions(SetResources(player, Subtract, TAX, Gas))
    EUDEndIf()


def run():
    global tick, paid_count
    tick += 1
    if EUDIf()(tick >= INTERVAL):
        tick << 0
        paid_count << 0
        for player in range(8):
            settle_player(player)
        if EUDIf()(paid_count >= 1):
            DoActions(DisplayText(Db("Relief payments issued.")))
        EUDEndIf()
    EUDEndIf()
