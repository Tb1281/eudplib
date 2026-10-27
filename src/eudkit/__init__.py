from __future__ import annotations

from collections.abc import Callable
from typing import Any, overload

from .ast_lowering import EudFunction, LoopRange, loop_range
from .session import MapContext, Session
from .types import (
    EPD,
     P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    Accumulate,
    ActionSpec,
    Add,
    Allies,
    AllPlayers,
    Always,
    AtLeast,
    AtMost,
    Bring,
    Cleared,
    Command,
    Comment,
    Condition,
    CreateUnit,
    CurrentPlayer,
    Deaths,
    DeathsX,
    Defeat,
    DisplayText,
    DisplayTextId,
    Exactly,
    Foes,
    Force1,
    Force2,
    Force3,
    Force4,
    Gas,
    MaskedVar,
    Memory,
    MemoryX,
    Never,
    NeutralPlayers,
    NonAlliedVictoryPlayers,
    Ore,
    OreAndGas,
    Player1,
    Player2,
    Player3,
    Player4,
    Player5,
    Player6,
    Player7,
    Player8,
    Player9,
    Player10,
    Player11,
    Player12,
    PreserveTrigger,
    Set,
    SetDeaths,
    SetDeathsX,
    SetMemory,
    SetMemoryX,
    SetResources,
    SetTo,
    Subtract,
    Switch,
    Var,
    Victory,
)


class _EudNamespace:
    @overload
    def fn(self, func: Callable[..., Any]) -> EudFunction: ...

    @overload
    def fn(
        self, *, main_loop: bool = True
    ) -> Callable[[Callable[..., Any]], EudFunction]: ...

    def fn(
        self,
        func: Callable[..., Any] | None = None,
        *,
        main_loop: bool = True,
    ) -> EudFunction | Callable[[Callable[..., Any]], EudFunction]:
        def _decorator(f: Callable[..., Any]) -> EudFunction:
            return EudFunction(f, main_loop=main_loop)

        if func is not None:
            return _decorator(func)
        return _decorator

    def loop_range(
        self, start: int, stop: int | None = None, step: int = 1
    ) -> LoopRange:
        return loop_range(start, stop, step)


eud = _EudNamespace()
fn = eud.fn

__all__ = [
    "EPD",
    "P1",
    "P2",
    "P3",
    "P4",
    "P5",
    "P6",
    "P7",
    "P8",
    "P9",
    "P10",
    "P11",
    "P12",
    "Accumulate",
    "ActionSpec",
    "Add",
    "AllPlayers",
    "Allies",
    "Always",
    "AtLeast",
    "AtMost",
    "Bring",
    "Cleared",
    "Command",
    "Comment",
    "Condition",
    "CreateUnit",
    "CurrentPlayer",
    "Deaths",
    "DeathsX",
    "Defeat",
    "DisplayText",
    "DisplayTextId",
    "EudFunction",
    "Exactly",
    "Foes",
    "Force1",
    "Force2",
    "Force3",
    "Force4",
    "Gas",
    "LoopRange",
    "MapContext",
    "MaskedVar",
    "Memory",
    "MemoryX",
    "Never",
    "NeutralPlayers",
    "NonAlliedVictoryPlayers",
    "Ore",
    "OreAndGas",
    "Player1",
    "Player2",
    "Player3",
    "Player4",
    "Player5",
    "Player6",
    "Player7",
    "Player8",
    "Player9",
    "Player10",
    "Player11",
    "Player12",
    "PreserveTrigger",
    "Session",
    "Set",
    "SetDeaths",
    "SetDeathsX",
    "SetMemory",
    "SetMemoryX",
    "SetResources",
    "SetTo",
    "Subtract",
    "Switch",
    "Var",
    "Victory",
    "eud",
    "fn",
    "loop_range",
]
