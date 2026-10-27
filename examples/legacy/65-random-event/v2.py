# 65 - probability event wheel (modern eudkit)
#
# Expresses ideal intent:
# 1. Periodic event clock via `with session.every(480):`.
# 2. Pure deterministic LCG random roll and range dispatch.
# 3. Deduplicated map strings and clean session state encapsulation.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

RHYNADON = 89
CENTER = 40
NEUTRAL = 11
PERIOD = 480
EVENT_COUNT = 4


def build(session: eudkit.Session):
    seed = session.var(123456789)
    roll = session.var(0)
    last_event = session.var(0)
    windfall_events = session.var(0)
    gas_events = session.var(0)
    tax_events = session.var(0)
    critter_events = session.var(0)
    quiet_events = session.var(0)
    total_windfall_ore = session.var(0)

    msg_windfall = session.map_string("Random event: mineral windfall!")
    msg_gas = session.map_string("Random event: gas rush!")
    msg_tax = session.map_string("Random event: tax day.")
    msg_critters = session.map_string("Random event: critters wander in.")

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal seed, roll, last_event, total_windfall_ore
        nonlocal windfall_events, gas_events, tax_events, critter_events, quiet_events

        with session.every(PERIOD):
            seed = seed * 1103515245 + 12345
            roll = (seed >> 16) % 100

            if roll < 15:
                for p in range(EVENT_COUNT):
                    eudkit.SetResources(p, eudkit.Add, 200, eudkit.Ore)
                total_windfall_ore += 200 * EVENT_COUNT
                windfall_events += 1
                last_event = 1
                msg_windfall.display()
            elif roll < 30:
                for p in range(EVENT_COUNT):
                    eudkit.SetResources(p, eudkit.Add, 100, eudkit.Gas)
                gas_events += 1
                last_event = 2
                msg_gas.display()
            elif roll < 45:
                for p in range(EVENT_COUNT):
                    eudkit.SetResources(p, eudkit.Subtract, 50, eudkit.Gas)
                tax_events += 1
                last_event = 3
                msg_tax.display()
            elif roll < 60:
                eudkit.CreateUnit(4, RHYNADON, CENTER, NEUTRAL)
                critter_events += 1
                last_event = 4
                msg_critters.display()
            else:
                quiet_events += 1
                last_event = 5

    return {
        "roll": roll,
        "last_event": last_event,
        "windfall_events": windfall_events,
        "gas_events": gas_events,
        "tax_events": tax_events,
        "critter_events": critter_events,
        "quiet_events": quiet_events,
        "total_windfall_ore": total_windfall_ore,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 65 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
