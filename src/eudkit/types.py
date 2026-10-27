from __future__ import annotations

import contextvars
from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING

from eudplib.bindings._rust import eudkit as _rust_eudkit

if TYPE_CHECKING:
    from .session import Session

_active_builder_session: contextvars.ContextVar[Session | None] = (
    contextvars.ContextVar("eudkit_active_builder_session", default=None)
)

# Player IDs
Player1 = 0
Player2 = 1
Player3 = 2
Player4 = 3
Player5 = 4
Player6 = 5
Player7 = 6
Player8 = 7
Player9 = 8
Player10 = 9
Player11 = 10
Player12 = 11
P1 = 0
P2 = 1
P3 = 2
P4 = 3
P5 = 4
P6 = 5
P7 = 6
P8 = 7
P9 = 8
P10 = 9
P11 = 10
P12 = 11
CurrentPlayer = 13
Foes = 14
Allies = 15
NeutralPlayers = 16
AllPlayers = 17
Force1 = 18
Force2 = 19
Force3 = 20
Force4 = 21
NonAlliedVictoryPlayers = 26

# Comparisons
AtLeast = 0
AtMost = 1
Exactly = 10

# Modifiers
SetTo = 7
Add = 8
Subtract = 9

# Resources
Ore = 0
Gas = 1
OreAndGas = 2

# Switch states
Set = 2
Cleared = 3

DEATHS_BASE_ADDR = 0x0058A364


def EPD(address: int) -> int:
    return (int(address) - DEATHS_BASE_ADDR) // 4


class Condition:
    """Represents one or more conjunctive StarCraft conditions (`AND`)."""

    def __init__(
        self,
        builders: Sequence[Callable[[Session], _rust_eudkit.RawCondition]],
    ) -> None:
        self._builders = list(builders)

    @classmethod
    def from_raw(cls, raw: _rust_eudkit.RawCondition) -> Condition:
        return cls([lambda _s: raw])

    def resolve(self, session: Session) -> list[_rust_eudkit.RawCondition]:
        return [b(session) for b in self._builders]

    def __and__(self, other: Condition) -> Condition:
        if not isinstance(other, Condition):
            return NotImplemented
        return Condition(self._builders + other._builders)


def coerce_conditions(
    cond: Condition | Sequence[Condition], session: Session
) -> list[_rust_eudkit.RawCondition]:
    if isinstance(cond, Condition):
        return cond.resolve(session)
    out: list[_rust_eudkit.RawCondition] = []
    for item in cond:
        if isinstance(item, Condition):
            out.extend(item.resolve(session))
        else:
            raise TypeError(f"Expected Condition, got {type(item).__name__}")
    return out


class MaskedVar:
    """Represents a bitwise masked view `(var & mask)` of a virtual variable."""

    def __init__(self, var: Var, mask: int) -> None:
        self.var = var
        self.mask = int(mask) & 0xFFFFFFFF

    def _cmp(self, comparison: int, value: int) -> Condition:
        vid = self.var.id
        vname = self.var.name
        vinit = self.var.init_val
        orig_session = self.var._session
        mask = self.mask
        val_u32 = int(value) & 0xFFFFFFFF

        def _build(session: Session) -> _rust_eudkit.RawCondition:
            target_id = (
                vid
                if session is orig_session
                else session._resolve_var_id(vname, vinit)
            )
            return _rust_eudkit.RawCondition.var_cmp(
                target_id, comparison, val_u32, mask
            )

        return Condition([_build])

    def __eq__(self, other: object) -> Condition:  # type: ignore[override]
        if not isinstance(other, int):
            raise TypeError("Variable comparison requires an integer constant")
        return self._cmp(Exactly, other)

    def __ne__(self, other: object) -> Condition:  # type: ignore[override]
        if not isinstance(other, int):
            raise TypeError("Variable comparison requires an integer constant")
        if (other & 0xFFFFFFFF) == 0:
            return self._cmp(AtLeast, 1)
        if (other & 0xFFFFFFFF) == 0xFFFFFFFF:
            return self._cmp(AtMost, 0xFFFFFFFE)
        raise ValueError(
            "Direct `!=` condition is only supported for 0 or 0xFFFFFFFF in a single condition"
        )

    def __ge__(self, other: int) -> Condition:
        return self._cmp(AtLeast, other)

    def __le__(self, other: int) -> Condition:
        return self._cmp(AtMost, other)

    def __gt__(self, other: int) -> Condition:
        return self._cmp(AtLeast, int(other) + 1)

    def __lt__(self, other: int) -> Condition:
        val = int(other)
        if val <= 0:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return self._cmp(AtMost, val - 1)

    def set(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, SetTo, session=session)

    def add(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, Add, session=session)

    def sub(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, Subtract, session=session)


class Var:
    """Handle to a virtual variable in an eudkit `Session` (`VarId` in HIR)."""

    def __init__(
        self, session: Session, var_id: int, name: str, init_val: int
    ) -> None:
        self._session = session
        self.id = var_id
        self.name = name
        self.init_val = int(init_val)

    def _target_session_and_id(
        self, session: Session | None = None
    ) -> tuple[Session, int]:
        active = session or _active_builder_session.get() or self._session
        if active is self._session:
            return active, self.id
        return active, active._resolve_var_id(self.name, self.init_val)

    def _cmp(self, comparison: int, value: int) -> Condition:
        vid = self.id
        vname = self.name
        vinit = self.init_val
        orig_session = self._session
        val_u32 = int(value) & 0xFFFFFFFF

        def _build(session: Session) -> _rust_eudkit.RawCondition:
            target_id = (
                vid
                if session is orig_session
                else session._resolve_var_id(vname, vinit)
            )
            return _rust_eudkit.RawCondition.var_cmp(
                target_id, comparison, val_u32, None
            )

        return Condition([_build])

    def __and__(self, mask: int) -> MaskedVar:
        return MaskedVar(self, mask)

    def __eq__(self, other: object) -> Condition:  # type: ignore[override]
        if not isinstance(other, int):
            raise TypeError("Variable comparison requires an integer constant")
        return self._cmp(Exactly, other)

    def __ne__(self, other: object) -> Condition:  # type: ignore[override]
        if not isinstance(other, int):
            raise TypeError("Variable comparison requires an integer constant")
        if (other & 0xFFFFFFFF) == 0:
            return self._cmp(AtLeast, 1)
        if (other & 0xFFFFFFFF) == 0xFFFFFFFF:
            return self._cmp(AtMost, 0xFFFFFFFE)
        raise ValueError(
            "Direct `!=` condition is only supported for 0 or 0xFFFFFFFF in a single condition"
        )

    def __ge__(self, other: int) -> Condition:
        return self._cmp(AtLeast, other)

    def __le__(self, other: int) -> Condition:
        return self._cmp(AtMost, other)

    def __gt__(self, other: int) -> Condition:
        return self._cmp(AtLeast, int(other) + 1)

    def __lt__(self, other: int) -> Condition:
        val = int(other)
        if val <= 0:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return self._cmp(AtMost, val - 1)

    def set(self, value: int, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        sess._raw.push_var_assign(vid, int(value))
        return self

    def assign(self, value: int, session: Session | None = None) -> Var:
        return self.set(value, session=session)

    def __lshift__(self, value: int) -> Var:
        return self.set(value)

    def add(self, value: int, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        val = int(value)
        if val >= 0:
            sess._raw.push_var_add(vid, val)
        else:
            sess._raw.push_var_sub(vid, -val)
        return self

    def sub(self, value: int, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        val = int(value)
        if val >= 0:
            sess._raw.push_var_sub(vid, val)
        else:
            sess._raw.push_var_add(vid, -val)
        return self

    def __iadd__(self, value: int) -> Var:
        return self.add(value)

    def __isub__(self, value: int) -> Var:
        return self.sub(value)

    def set_mask(
        self,
        value: int,
        mask: int,
        modifier: int = SetTo,
        session: Session | None = None,
    ) -> Var:
        sess, vid = self._target_session_and_id(session)
        sess._raw.push_var_set_mask(
            vid, int(modifier), int(value), int(mask) & 0xFFFFFFFF
        )
        return self


class ActionSpec:
    """Represents a StarCraft trigger action (either emitted in a builder scope or passed to `session.trigger`)."""

    def __init__(
        self, builder: Callable[[Session], _rust_eudkit.RawAction]
    ) -> None:
        self._builder = builder
        active = _active_builder_session.get()
        if active is not None:
            active._raw.push_action(self.resolve(active))

    def resolve(self, session: Session) -> _rust_eudkit.RawAction:
        return self._builder(session)


def _resolve_unit(session: Session, unit: int | str) -> int:
    if isinstance(unit, str):
        return session.map_context.unit_id(unit)
    return int(unit)


def _resolve_location(session: Session, location: int | str) -> int:
    if isinstance(location, str):
        return session.map_context.location_id(location)
    return int(location)


def _resolve_switch(session: Session, switch: int | str) -> int:
    if isinstance(switch, str):
        return session.map_context.switch_id(switch)
    return int(switch)


# --- Stock Conditions ---


def Always() -> Condition:
    return Condition.from_raw(_rust_eudkit.RawCondition.always())


def Never() -> Condition:
    return Condition.from_raw(_rust_eudkit.RawCondition.never())


def Deaths(
    player: int, comparison: int, amount: int, unit: int | str = 0
) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.deaths(
                int(player),
                int(comparison),
                int(amount) & 0xFFFFFFFF,
                _resolve_unit(s, unit),
            )
        ]
    )


def DeathsX(
    player: int,
    comparison: int,
    amount: int,
    unit: int | str = 0,
    mask: int = 0xFFFFFFFF,
) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.deaths_x(
                int(player),
                int(comparison),
                int(amount) & 0xFFFFFFFF,
                _resolve_unit(s, unit),
                int(mask) & 0xFFFFFFFF,
            )
        ]
    )


def Memory(address: int, comparison: int, value: int) -> Condition:
    return Condition.from_raw(
        _rust_eudkit.RawCondition.memory(
            int(address) & 0xFFFFFFFF,
            int(comparison),
            int(value) & 0xFFFFFFFF,
        )
    )


def MemoryX(address: int, comparison: int, value: int, mask: int) -> Condition:
    return Condition.from_raw(
        _rust_eudkit.RawCondition.memory_x(
            int(address) & 0xFFFFFFFF,
            int(comparison),
            int(value) & 0xFFFFFFFF,
            int(mask) & 0xFFFFFFFF,
        )
    )


def Accumulate(
    player: int, comparison: int, amount: int, resource: int = Ore
) -> Condition:
    return Condition.from_raw(
        _rust_eudkit.RawCondition.accumulate(
            int(player),
            int(comparison),
            int(amount) & 0xFFFFFFFF,
            int(resource),
        )
    )


def Command(
    player: int, comparison: int, amount: int, unit: int | str = 0
) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.command(
                int(player),
                int(comparison),
                int(amount) & 0xFFFFFFFF,
                _resolve_unit(s, unit),
            )
        ]
    )


def Bring(
    player: int,
    comparison: int,
    amount: int,
    unit: int | str,
    location: int | str,
) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.bring(
                int(player),
                int(comparison),
                int(amount) & 0xFFFFFFFF,
                _resolve_unit(s, unit),
                _resolve_location(s, location),
            )
        ]
    )


def Switch(switch: int | str, state: int = Set) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.switch(
                _resolve_switch(s, switch), int(state)
            )
        ]
    )


# --- Stock Actions ---


def SetDeaths(
    player: int, modifier: int, value: int, unit: int | str = 0
) -> ActionSpec:
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_deaths(
            int(player), int(modifier), int(value), _resolve_unit(s, unit)
        )
    )


def SetDeathsX(
    player: int,
    modifier: int,
    value: int,
    unit: int | str = 0,
    mask: int = 0xFFFFFFFF,
) -> ActionSpec:
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_deaths_x(
            int(player),
            int(modifier),
            int(value),
            _resolve_unit(s, unit),
            int(mask) & 0xFFFFFFFF,
        )
    )


def SetMemory(address: int, modifier: int, value: int) -> ActionSpec:
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.set_memory(
            int(address) & 0xFFFFFFFF, int(modifier), int(value)
        )
    )


def SetMemoryX(
    address: int, modifier: int, value: int, mask: int
) -> ActionSpec:
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.set_memory_x(
            int(address) & 0xFFFFFFFF,
            int(modifier),
            int(value),
            int(mask) & 0xFFFFFFFF,
        )
    )


def SetResources(
    player: int, modifier: int, amount: int, resource: int = Ore
) -> ActionSpec:
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.set_resources(
            int(player), int(modifier), int(amount) & 0xFFFFFFFF, int(resource)
        )
    )


def DisplayText(text: str, always_display: bool = True) -> ActionSpec:
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.display_text(
            str(text), bool(always_display)
        )
    )


def DisplayTextId(string_id: int, always_display: bool = True) -> ActionSpec:
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.display_text_id(
            int(string_id), bool(always_display)
        )
    )


def CreateUnit(
    count: int, unit: int | str, location: int | str, player: int
) -> ActionSpec:
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.create_unit(
            int(count),
            _resolve_unit(s, unit),
            _resolve_location(s, location),
            int(player),
        )
    )


def Comment(text: str) -> ActionSpec:
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.comment(str(text)))


def PreserveTrigger() -> ActionSpec:
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.preserve_trigger())


def Victory() -> ActionSpec:
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.victory())


def Defeat() -> ActionSpec:
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.defeat())
