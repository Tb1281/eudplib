from __future__ import annotations

from collections.abc import Callable
from typing import Any, overload

from .ast_lowering import EudFuncPtr, EudFunction, LoopRange, loop_range
from .session import MapContext, Session, session
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
    Deaths,
    DeathsX,
    Defeat,
    DisplayText,
    DisplayTextId,
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
    String,
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
    bread,
    bread_epd,
    bwrite,
    bwrite_epd,
    displayText,
    display_at,
    DisplayTextAt,
    dwread,
    dwread_epd,
    dwwrite,
    dwwrite_epd,
    epdread_epd,
    f_eprintln,
    print_at,
    wread,
    wread_epd,
    wwrite,
    wwrite_epd,
    KillUnitAt,
    MinimapPing,
    Clear,
    Toggle,
    Random,
    Total,
    Units,
    Buildings,
    UnitsAndBuildings,
    Kills,
    Razings,
    KillsAndRazings,
    Custom,
    ElapsedTime,
    Score,
    SetScore,
    SetSwitch,
    MoveLocation,
    RemoveUnitAt,
    KillUnit,
    MoveUnit,
    GiveUnits,
    Order,
    PlayWAV,
    Move,
    Patrol,
    Attack,
    All,
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
        """Decorator that compiles a Python function into an eudkit trigger function.

        Use ``@eud.fn`` for the default form, or ``@eud.fn(main_loop=...,
        inline=...)`` to control lowering. A zero-argument function with no
        explicit ``inline`` setting becomes a main-loop function; other
        functions are callable from eudkit code and are lowered into the active
        compilation session.
        """
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
        """Allocates an indirect function pointer in the active session.

        ``param_count`` and ``return_count`` describe the target function's
        signature. ``init`` optionally binds an initial function, and ``name``
        labels the pointer in HIR; use ``.set(fn)`` to retarget it later.
        """
        from .types import _active_builder_session

        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("No active session found for func_ptr")
        return sess.func_ptr(param_count, return_count, init=init, name=name)

    def loop_range(
        self, start: int, stop: int | None = None, step: int = 1
    ) -> LoopRange:
        """Marks a runtime, trigger-lowered loop range inside ``@eud.fn``.

        A ``for`` over a ``loop_range`` becomes a ``while`` trigger loop instead
        of being unrolled at compile time. Accepts ``loop_range(stop)`` or
        ``loop_range(start, stop, step)`` mirroring Python's ``range``.
        """
        return loop_range(start, stop, step)

    def do(self, *actions: Any) -> None:
        """Emits one or more stock actions into the active session's current block.

        Equivalent to ``session.do(*actions)``. Raises ``RuntimeError`` when no
        builder session is active.
        """
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
    "Deaths",
    "DeathsX",
    "Defeat",
    "DisplayText",
    "DisplayTextId",
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
    "session",
    "Set",
    "SetDeaths",
    "SetDeathsX",
    "SetMemory",
    "SetMemoryX",
    "SetResources",
    "SetTo",
    "String",
    "StringBuffer",
    "Subtract",
    "Switch",
    "VArray",
    "Var",
    "Victory",
    "bread",
    "bread_epd",
    "bwrite",
    "bwrite_epd",
    "displayText",
    "do",
    "dwread",
    "dwread_epd",
    "dwwrite",
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
    "display_at",
    "DisplayTextAt",
    "print_at",
    "wread",
    "wread_epd",
    "wwrite",
    "wwrite_epd",
    "KillUnitAt",
    "MinimapPing",
    "Clear",
    "Toggle",
    "Random",
    "Total",
    "Units",
    "Buildings",
    "UnitsAndBuildings",
    "Kills",
    "Razings",
    "KillsAndRazings",
    "Custom",
    "ElapsedTime",
    "Score",
    "SetScore",
    "SetSwitch",
    "MoveLocation",
    "RemoveUnitAt",
    "KillUnit",
    "MoveUnit",
    "GiveUnits",
    "Order",
    "PlayWAV",
    "Move",
    "Patrol",
    "Attack",
    "All",
]
