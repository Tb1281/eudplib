from __future__ import annotations

from collections.abc import Callable
from typing import Any, overload

from .ast_lowering import EudFuncPtr, EudFunction, LoopRange, loop_range
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
    Array,
    AtLeast,
    AtMost,
    Bring,
    CSprite,
    CUnit,
    Cleared,
    Command,
    Comment,
    Condition,
    CreateUnit,
    CurrentPlayer,
    DBString,
    Deaths,
    DeathsX,
    Defeat,
    DisplayText,
    DisplayTextId,
    EUDString,
    Exactly,
    Expr,
    Foes,
    Force1,
    Force2,
    Force3,
    Force4,
    Gas,
    IsUserCP,
    MapString,
    MaskedVar,
    StringBuffer,
    Memory,
    MemoryX,
    Never,
    NeutralPlayers,
    NonAlliedVictoryPlayers,
    after,
    every,
    once,
    until,
    when,
    Ore,
    OreAndGas,
    Player,
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
    VArray,
    Var,
    Victory,
    bread_epd,
    bwrite_epd,
    displayText,
    dwread_epd,
    dwwrite_epd,
    epdread_epd,
    f_eprintln,
    print_at,
    wread_epd,
    wwrite_epd,
)


class _EudNamespace:
    @overload
    def fn(self, func: Callable[..., Any]) -> EudFunction: ...

    @overload
    def fn(
        self, *, main_loop: bool | None = None, inline: bool | None = None
    ) -> Callable[[Callable[..., Any]], EudFunction]: ...

    def fn(
        self,
        func: Callable[..., Any] | None = None,
        *,
        main_loop: bool | None = None,
        inline: bool | None = None,
    ) -> EudFunction | Callable[[Callable[..., Any]], EudFunction]:
        def _decorator(f: Callable[..., Any]) -> EudFunction:
            return EudFunction(f, main_loop=main_loop, inline=inline)

        if func is not None:
            return _decorator(func)
        return _decorator

    def func_ptr(
        self,
        param_count: int = 0,
        return_count: int = 0,
        *,
        init: EudFunction | None = None,
        name: str | None = None,
    ) -> EudFuncPtr:
        from .types import _active_builder_session

        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("No active session found for func_ptr")
        return sess.func_ptr(param_count, return_count, init=init, name=name)

    def loop_range(
        self, start: int, stop: int | None = None, step: int = 1
    ) -> LoopRange:
        return loop_range(start, stop, step)

    def do(self, *actions: Any) -> None:
        from .types import _active_builder_session

        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("No active session found for eud.do")
        sess.do(*actions)


eud = _EudNamespace()
fn = eud.fn
func_ptr = eud.func_ptr
do = eud.do

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
    "Array",
    "AtLeast",
    "AtMost",
    "Bring",
    "CSprite",
    "CUnit",
    "Cleared",
    "Command",
    "Comment",
    "Condition",
    "CreateUnit",
    "CurrentPlayer",
    "DBString",
    "Deaths",
    "DeathsX",
    "Defeat",
    "DisplayText",
    "DisplayTextId",
    "EUDString",
    "EudFuncPtr",
    "EudFunction",
    "Exactly",
    "Expr",
    "Foes",
    "Force1",
    "Force2",
    "Force3",
    "Force4",
    "Gas",
    "IsUserCP",
    "LoopRange",
    "MapContext",
    "MapString",
    "MaskedVar",
    "Memory",
    "MemoryX",
    "Never",
    "NeutralPlayers",
    "NonAlliedVictoryPlayers",
    "Ore",
    "OreAndGas",
    "Player",
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
    "StringBuffer",
    "Subtract",
    "Switch",
    "VArray",
    "Var",
    "Victory",
    "bread_epd",
    "bwrite_epd",
    "displayText",
    "do",
    "dwread_epd",
    "dwwrite_epd",
    "epdread_epd",
    "eud",
    "f_eprintln",
    "fn",
    "func_ptr",
    "loop_range",
    "after",
    "every",
    "once",
    "until",
    "when",
    "print_at",
    "wread_epd",
    "wwrite_epd",
]
