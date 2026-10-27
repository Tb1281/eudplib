from __future__ import annotations

import contextlib
from collections.abc import Callable, Iterator, Sequence
from pathlib import Path
from typing import Any, overload

from eudplib.bindings._rust import eudkit as _rust_eudkit

from .ast_lowering import EudFuncPtr, EudFunction
from .types import (
    ActionSpec,
    Condition,
    MapString,
    Var,
    _active_builder_session,
    coerce_conditions,
    displayText,
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

    def f_eprintln(self, *args: Any, always_display: bool = True) -> None:
        """Formats and displays text using CPString streaming actions."""
        self.do(f_eprintln(*args, always_display=always_display))

    def displayText(self, *args: Any, always_display: bool = True) -> None:
        """Displays static or formatted text."""
        self.do(displayText(*args, always_display=always_display))

    def print_at(self, line: int, *args: Any) -> None:
        """Formats and writes text to a specific screen line (0..12)."""
        self.do(print_at(line, *args))

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
    def main_loop(self, name: str = "main") -> Iterator[Session]:
        tok = _active_builder_session.set(self)
        self._raw.begin_function(name, True)
        try:
            yield self
        finally:
            self._raw.end_function()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def if_(self, conditions: Condition | Sequence[Condition]) -> Iterator[None]:
        tok = _active_builder_session.set(self)
        self._raw.begin_if(coerce_conditions(conditions, self))
        try:
            yield
        finally:
            self._raw.end_if()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def elif_(
        self, conditions: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        tok = _active_builder_session.set(self)
        self._raw.begin_elif(coerce_conditions(conditions, self))
        try:
            yield
        finally:
            self._raw.end_elif()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def else_(self) -> Iterator[None]:
        tok = _active_builder_session.set(self)
        self._raw.begin_else()
        try:
            yield
        finally:
            self._raw.end_else()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def while_(
        self, conditions: Condition | Sequence[Condition]
    ) -> Iterator[None]:
        tok = _active_builder_session.set(self)
        self._raw.begin_while(coerce_conditions(conditions, self))
        try:
            yield
        finally:
            self._raw.end_while()
            _active_builder_session.reset(tok)

    @contextlib.contextmanager
    def once(
        self, conditions: Condition | Sequence[Condition] | None = None
    ) -> Iterator[None]:
        """Scoped one-time block executed exactly once (optionally guarded by conditions)."""
        guard = self.var(1)
        if conditions is None:
            with self.if_(guard == 1):
                guard.set(0, session=self)
                yield
        else:
            cond_list = list(conditions) if isinstance(conditions, (list, tuple)) else [conditions]
            cond_list.append(guard == 1)
            with self.if_(cond_list):
                guard.set(0, session=self)
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

    def dump_lir(self) -> str:
        return self._raw.dump_lir()

    def compile(
        self, output_map: str | Path
    ) -> _rust_eudkit.CompilationReport:
        return self._raw.compile(str(output_map))
