# 54 - item slots (modern eudkit)
#
# Expresses ideal intent:
# 1. 4-slot hero belt using EPD-addressed `Array(4)` storage.
# 2. Declarative reactive timers via `with every(...):` for drops and damage.
# 3. First-class `Switch` conditions and `SetSwitch` actions.
from pathlib import Path
import eudkit

BASEMAP_PATH = Path(__file__).resolve().parents[3] / "tests" / "outputmap" / "basemap" / "basemap_strx.scx"

EMPTY = 0
POTION = 1
ELIXIR = 2
SLOTS = 4
HERO_MAX_HP = 1200
HERO_MAX_MANA = 100
POTION_HEAL = 300
POTION_PERIOD = 90
ELIXIR_PERIOD = 270
DAMAGE_PERIOD = 30
AMBIENT_DAMAGE = 7

SW_USE_POTION = 4
SW_USE_ELIXIR = 5
SW_DROP = 6


def build(session: eudkit.Session):
    slots = eudkit.Array(SLOTS)
    hero_hp = session.var(900)
    hero_mana = session.var(40)
    potions_used = session.var(0)
    elixirs_used = session.var(0)
    drops = session.var(0)
    failed_uses = session.var(0)
    potions_found = session.var(0)
    elixirs_found = session.var(0)

    @session.fn(main_loop=True)
    def game_loop() -> None:
        nonlocal hero_hp, hero_mana, potions_used, elixirs_used, drops
        nonlocal failed_uses, potions_found, elixirs_found

        # 1. Ambient damage
        with eudkit.every(DAMAGE_PERIOD):
            if hero_hp >= AMBIENT_DAMAGE + 1:
                hero_hp -= AMBIENT_DAMAGE

        # 2. Potion drop
        with eudkit.every(POTION_PERIOD):
            found_slot = -1
            for i in range(4):
                if slots[i] == EMPTY:
                    if found_slot == -1:
                        found_slot = i
            if found_slot != -1:
                slots[found_slot] = POTION
                potions_found += 1

        # 3. Elixir drop
        with eudkit.every(ELIXIR_PERIOD):
            found_slot = -1
            for i in range(4):
                if slots[i] == EMPTY:
                    if found_slot == -1:
                        found_slot = i
            if found_slot != -1:
                slots[found_slot] = ELIXIR
                elixirs_found += 1

        # 4. Use potion
        if eudkit.Switch(SW_USE_POTION, eudkit.Set):
            eudkit.SetSwitch(SW_USE_POTION, eudkit.Clear)
            found_slot = -1
            for i in range(4):
                if slots[i] == POTION:
                    if found_slot == -1:
                        found_slot = i
            if found_slot != -1:
                if hero_hp <= HERO_MAX_HP - POTION_HEAL:
                    hero_hp += POTION_HEAL
                else:
                    hero_hp = HERO_MAX_HP
                slots[found_slot] = EMPTY
                potions_used += 1
            else:
                failed_uses += 1

        # 5. Use elixir
        if eudkit.Switch(SW_USE_ELIXIR, eudkit.Set):
            eudkit.SetSwitch(SW_USE_ELIXIR, eudkit.Clear)
            found_slot = -1
            for i in range(4):
                if slots[i] == ELIXIR:
                    if found_slot == -1:
                        found_slot = i
            if found_slot != -1:
                hero_hp = HERO_MAX_HP
                hero_mana = HERO_MAX_MANA
                slots[found_slot] = EMPTY
                elixirs_used += 1
            else:
                failed_uses += 1

        # 6. Drop item
        if eudkit.Switch(SW_DROP, eudkit.Set):
            eudkit.SetSwitch(SW_DROP, eudkit.Clear)
            drop_slot = -1
            for i in range(3, -1, -1):
                if slots[i] != EMPTY:
                    if drop_slot == -1:
                        drop_slot = i
            if drop_slot != -1:
                slots[drop_slot] = EMPTY
                drops += 1
            else:
                failed_uses += 1

    return {
        "hero_hp": hero_hp,
        "hero_mana": hero_mana,
        "potions_used": potions_used,
        "elixirs_used": elixirs_used,
        "drops": drops,
    }


def main():
    session = eudkit.Session(BASEMAP_PATH)
    build(session)
    out_path = Path(__file__).parent / "output_v2.scx"
    report = session.compile(out_path)
    if out_path.exists():
        out_path.unlink()
    print("Scenario 54 (v2) compiled successfully:")
    print(f"  Triggers: {report.trigger_count}")
    print(f"  Payload:  {report.payload_size} bytes")
    print(f"  VProcs:   {report.vproc_count}")
    return report


if __name__ == "__main__":
    main()
