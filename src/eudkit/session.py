from __future__ import annotations

import contextlib
from collections.abc import Callable, Iterator, Sequence
from pathlib import Path
from typing import TYPE_CHECKING, Any, overload

from eudkit import _rust as _rust_eudkit

from .ast_lowering import EudFuncPtr, EudFunction
from .types import (
    ActionSpec,
    Condition,
    MapString,
    StringBuffer,
    Var,
    _active_builder_session,
    coerce_conditions,
    displayText,
    display_at,
    f_eprintln,
    print_at,
)


class MapContext:
    """Read-only view and symbol resolver for map assets extracted from an input `.scx`/`.scm`."""

    def __init__(self, raw_session: _rust_eudkit.RawSession) -> None:
        self._raw = raw_session

    def location_id(self, name: str) -> int:
        return self._raw.location_id(name)

    def unit_id(self, name: str) -> int:
        return self._raw.unit_id(name)

    def switch_id(self, name: str) -> int:
        return self._raw.switch_id(name)

    def get_or_insert_string(self, text: str) -> int:
        return self._raw.get_or_insert_string(text)

    @property
    def classic_trigger_count(self) -> int:
        return self._raw.classic_trigger_count()

    @property
    def locations_count(self) -> int:
        return self._raw.locations_count()

    @property
    def placed_units_count(self) -> int:
        return self._raw.placed_units_count()


class Session:
    """Isolated compilation session holding MapContext, HIR/LIR, and configuration with zero global state."""

    def __init__(
        self,
        input_map: str | Path,
        *,
        obscurate: bool = False,
        obscurate_seed: int | None = None,
        preserve_classic_triggers: bool = True,
    ) -> None:
        self.input_map_path = Path(input_map)
        self._raw = _rust_eudkit.RawSession(
            str(self.input_map_path),
            obscurate=obscurate,
            obscurate_seed=obscurate_seed,
            preserve_classic_triggers=preserve_classic_triggers,
        )
        self.map_context = MapContext(self._raw)
        self._var_counter = 0
        self._buf_counter = 0
        self._fp_counter = 0
        self._vars_by_name: dict[str, Var] = {}
        self._compiled_report: _rust_eudkit.CompilationReport | None = None

    def __enter__(self) -> Session:
        self._ctx_token = _active_builder_session.set(self)
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_ctx_token"):
            _active_builder_session.reset(self._ctx_token)

    def set_obscurate(self, obscurate: bool = True, *, seed: int | None = None) -> None:
        """Enables or disables seeded Obscuration pass on this Compilation Session."""
        self._raw.set_obscurate(obscurate, seed=seed)

    def set_obscurate_seed(self, seed: int) -> None:
        """Sets the seed for the Obscuration pass."""
        self._raw.set_obscurate_seed(seed)

    def var(self, init: int = 0, *, name: str | None = None) -> Var:
        if name is None:
            name = f"v{self._var_counter}"
        self._var_counter += 1
        vid = self._raw.add_var(name, int(init))
        v = Var(self, vid, name, int(init))
        self._vars_by_name[name] = v
        return v

    def _resolve_var_id(self, name: str, init_val: int) -> int:
        if name in self._vars_by_name:
            return self._vars_by_name[name].id
        return self.var(init_val, name=name).id

    def byte_buf(self, data: bytes, *, name: str | None = None) -> int:
        if name is None:
            name = f"buf{self._buf_counter}"
        self._buf_counter += 1
        return self._raw.add_byte_buf(name, bytes(data))

    def map_string(self, text: str) -> MapString:
        """Registers or retrieves a MapString inside this Compilation Session."""
        return MapString(text, session=self)

    def string(self, content: str | int = 1024) -> StringBuffer:
        """Allocates or registers a format-capable string inside this Compilation Session."""
        return StringBuffer(content, session=self)

    def string_buffer(self, content: str | int = 1024) -> StringBuffer:
        """Deprecated alias for session.string(). Use session.string() instead."""
        return self.string(content)

    def f_eprintln(self, *args: Any, always_display: bool = True) -> None:
        """Formats and displays text using CPString streaming actions."""
        self.do(f_eprintln(*args, always_display=always_display))

    def displayText(self, *args: Any, always_display: bool = True) -> None:
        """Displays static or formatted text."""
        self.do(displayText(*args, always_display=always_display))

    def display_at(self, line: int, *args: Any) -> None:
        """Formats and writes text to a fixed on-screen HUD line (0..12)."""
        self.do(display_at(line, *args))

    print_at = display_at

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
            ef = EudFunction(f, session=self, main_loop=main_loop, inline=inline)
            ef.lower_into(self, main_loop=main_loop, inline=inline)
            return ef

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
        if name is None:
            name = f"fp{self._fp_counter}"
            self._fp_counter += 1
        if init is not None and init.fn_id is None:
            init.lower_into(self)
        init_id = init.fn_id if (init is not None and init.fn_id is not None) else None
        ptr_id = self._raw.add_func_ptr(name, param_count, return_count, init_id)
        return EudFuncPtr(self, ptr_id, name, param_count, return_count)

    def add_fn(
        self,
        func: EudFunction | Callable[..., Any],
        *,
        main_loop: bool | None = None,
        inline: bool | None = None,
    ) -> int:
        if isinstance(func, EudFunction):
            return func.lower_into(self, main_loop=main_loop, inline=inline)
        ef = EudFunction(func, session=self, main_loop=main_loop, inline=inline)
        return ef.lower_into(self, main_loop=main_loop, inline=inline)

    @contextlib.contextmanager
    def _scope(
        self,
        begin: Callable[[], None],
        end: Callable[[], None],
    ) -> Iterator[None]:
        """Set the ambient session around one raw begin/end block."""
        tok = _active_builder_session.set(self)
        begin()
        try:
            yield
        finally:
            end()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def main_loop(self, name: str = "main") -> Iterator[Session]:
        with self._scope(
            lambda: self._raw.begin_function(name, True),
            lambda: self._raw.end_function(),
        ):
            yield self

    @contextlib.contextmanager
    def if_(self, conditions: Condition | Sequence[Condition]) -> Iterator[None]:
        with self._scope(
            lambda: self._raw.begin_if(coerce_conditions(conditions, self)),
            self._raw.end_if,
        ):
            yield

    @contextlib.contextmanager
    def elif_(
        self, conditions: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        with self._scope(
            lambda: self._raw.begin_elif(coerce_conditions(conditions, self)),
            self._raw.end_elif,
        ):
            yield

    @contextlib.contextmanager
    def else_(self) -> Iterator[None]:
        with self._scope(self._raw.begin_else, self._raw.end_else):
            yield

    @contextlib.contextmanager
    def while_(
        self, conditions: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        with self._scope(
            lambda: self._raw.begin_while(coerce_conditions(conditions, self)),
            self._raw.end_while,
        ):
            yield

    @contextlib.contextmanager
    def once(
        self, conditions: Condition | Sequence[Condition] | None = None
    ) -> Iterator[None]:
        """Scoped one-time rule executed exactly once.
        
        If `conditions` is provided, the block waits until the conditions hold,
        executes once, and permanently deactivates its guard.
        If `conditions` is None, executes on the very first trigger loop and permanently deactivates.
        """
        with self._scope(
            lambda: self._raw.begin_once(
                coerce_conditions(conditions, self) if conditions is not None else []
            ),
            self._raw.end_reactive,
        ):
            yield

    @contextlib.contextmanager
    def when(
        self, conditions: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        """Standing reactive rule: executes every trigger loop while conditions evaluate to true.
        
        In StarCraft's polling-based trigger VM, `when(cond)` evaluates conditions at the start
        of each trigger loop pass. As long as the conditions remain true, the enclosed actions execute.
        """
        with self._scope(
            lambda: self._raw.begin_when(coerce_conditions(conditions, self)),
            self._raw.end_reactive,
        ):
            yield

    @contextlib.contextmanager
    def until(
        self, condition: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        """Continuous rule: executes every trigger loop until condition holds, then permanently deactivates.
        
        Evaluated each trigger loop cycle. As soon as `condition` becomes true, the guard flips
        and the block will never execute again in subsequent trigger loops.
        """
        with self._scope(
            lambda: self._raw.begin_until(coerce_conditions(condition, self)),
            self._raw.end_reactive,
        ):
            yield

    @contextlib.contextmanager
    def after(
        self, loops: int
    ) -> Iterator[None]:
        """Delay rule: countdown loop counter and fire once after `loops` trigger cycles have elapsed.
        
        StarCraft Trigger Loop Architecture:
        In Brood War, triggers do NOT run every frame by default; they run in discrete
        Trigger Loops (roughly every 30-31 game frames in standard maps, or every single
        frame when an EUD Turbo Trigger / Hyper Trigger continuously resets the trigger loop tick
        countdown to 0).
        
        Native `Wait(ms)` actions freeze the player's trigger evaluation thread and cause multiplayer desyncs.
        `after(loops)` avoids `Wait` entirely by maintaining a deterministic 4-byte counter variable
        decremented on each trigger loop pass. Once the specified number of trigger loops has elapsed,
        the body executes once and deactivates.
        
        :param loops: Number of trigger loop evaluation cycles to wait before executing (e.g. 24 loops = ~1 second under Turbo).
        """
        with self._scope(
            lambda: self._raw.begin_after(int(loops)),
            self._raw.end_reactive,
        ):
            yield

    @contextlib.contextmanager
    def every(
        self, loops: int
    ) -> Iterator[None]:
        """Periodic clock rule: fires repeatedly every `loops` trigger cycles.
        
        StarCraft Trigger Loop Architecture:
        In Brood War, triggers execute in discrete Trigger Loops. In standard maps, 1 loop
        occurs every ~31 frames (~1.3s on Fastest). When an EUD Turbo Trigger is present,
        the engine's trigger countdown tick is kept at 0, causing the trigger loop to execute
        every single game frame (24 times/second on Fastest).
        
        `every(loops)` maintains a deterministic countdown variable that decrements on each trigger
        loop cycle and resets to `loops` upon hitting 0, executing the body without relying on
        fragile or desync-prone native `Wait()` actions.
        
        :param loops: Interval in trigger loop cycles between successive firings (e.g. 24 loops = every 1s under Turbo).
        """
        with self._scope(
            lambda: self._raw.begin_every(int(loops)),
            self._raw.end_reactive,
        ):
            yield

    def break_(self) -> None:
        self._raw.push_break()

    def continue_(self) -> None:
        self._raw.push_continue()

    def do(self, *actions: ActionSpec) -> None:
        for act in actions:
            self._raw.push_action(act.resolve(self))

    def trigger(
        self,
        conditions: Condition | Sequence[Condition],
        actions: ActionSpec | Sequence[ActionSpec],
        *,
        label: str | None = None,
        preserved: bool = True,
    ) -> None:
        raw_conds = coerce_conditions(conditions, self)
        act_list = [actions] if isinstance(actions, ActionSpec) else list(actions)
        raw_acts = [a.resolve(self) for a in act_list]
        self._raw.push_trigger(
            raw_conds, raw_acts, label=label, preserved=preserved
        )

    def strx_byte_len(self) -> int:
        return self._raw.strx_byte_len()

    def dump_hir(self) -> str:
        return self._raw.dump_hir()

    def dump_plan(self) -> str:
        """Returns the canonical, versioned HIR plan for this session."""
        return self._raw.dump_plan()

    def write_plan(self, output_path: str | Path) -> None:
        """Writes the canonical HIR plan artifact (`.eudhir`) for this session."""
        self._raw.write_plan(str(output_path))

    def dump_lir(self) -> str:
        return self._raw.dump_lir()

    def compile(
        self, output_map: str | Path
    ) -> _rust_eudkit.CompilationReport:
        report = self._raw.compile(str(output_map))
        self._compiled_report = report
        return report


class AmbientSessionProxy:
    """Proxy object that dynamically delegates to the active compilation session in the current context."""

    def _get_active(self) -> Session:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError(
                "No active eudkit compilation session found in the current context.\n"
                "Scripts using ambient 'session' must be executed via 'eudkit build' "
                "or within an active 'with Session(...):' context."
            )
        return sess

    def __getattr__(self, name: str) -> Any:
        return getattr(self._get_active(), name)

    def __repr__(self) -> str:
        sess = _active_builder_session.get()
        if sess is None:
            return "<AmbientSessionProxy (no active session)>"
        return f"<AmbientSessionProxy active={sess!r}>"


if TYPE_CHECKING:
    session: Session
else:
    session = AmbientSessionProxy()
