"""Example 05: Strings & Display — Modern eudkit (v2).

Demonstrates:
- Canonical `session.string(content)` factory for mutable in-game formatted strings
- Deduplicated MapString resolution at compile time
- Residue-stride CPString streaming actions (mod 12 unit-stride grouping)
- Static STRx relocation (0x191943C8)
- f_eprintln template formatting (%d, %s) with 0 runtime decryption overhead
- IsUserCP condition checking
- Standalone compile via `eudkit build`
"""

from eudkit import IsUserCP, f_eprintln, session

# 1. Deduplicated MapString:
# Identical strings registered in session scope share the exact same STRx table offset
ms1 = session.map_string("Mission Objective: Destroy Enemy Nexus")
ms2 = session.map_string("Mission Objective: Destroy Enemy Nexus")
assert ms1.id == ms2.id

# 2. Canonical dynamic formatted String in STRx:
hud_str = session.string(64)

score = session.var(100, name="score")
minerals = session.var(500, name="minerals")
gas = session.var(250, name="gas")


@session.fn
def game_loop():
    # Dynamic in-game string formatting
    hud_str.printf("Score: %d | Min: %d", score, minerals)
    hud_str.display()

    # 3. Template string formatting with residue-stride CPString writes:
    # Instead of 150KB runtime loops, eudkit streams characters directly into the
    # StarCraft text buffer using grouped unit-stride CurrentPlayer writes.
    f_eprintln("Status Report: Score = %d", score)
    f_eprintln("Resources: Min = %d, Gas = %d", minerals, gas)

    # 4. Local player filtering
    if IsUserCP():
        f_eprintln("Private Dispatch: Active Player Score = %d", score)
