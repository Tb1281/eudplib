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


def _to_i32(val: int) -> int:
    u = int(val) & 0xFFFFFFFF
    return u - 0x100000000 if u >= 0x80000000 else u


# Binary op codes matching `decode_bin_op` in `eudkit_py.rs`
_BINOP_ADD = 0
_BINOP_SUB = 1
_BINOP_MUL = 2
_BINOP_DIV = 3
_BINOP_REM = 4
_BINOP_BITAND = 5
_BINOP_BITOR = 6
_BINOP_BITXOR = 7
_BINOP_SHL = 8
_BINOP_SHR = 9
_BINOP_SUBSAT = 10


def _is_expr_like(val: object) -> bool:
    return isinstance(val, (int, Expr, Var, MaskedVar))


def _coerce_raw_expr(
    val: ExprLike, session: Session
) -> _rust_eudkit.RawExpr:
    if isinstance(val, int):
        return _rust_eudkit.RawExpr.const_val(_to_i32(val))
    if isinstance(val, Expr):
        return val.resolve(session)
    if isinstance(val, Var):
        target_id = (
            val.id
            if session is val._session
            else session._resolve_var_id(val.name, val.init_val)
        )
        return _rust_eudkit.RawExpr.var(target_id)
    if isinstance(val, MaskedVar):
        var_expr = _coerce_raw_expr(val.var, session)
        mask_expr = _rust_eudkit.RawExpr.const_val(_to_i32(val.mask))
        return _rust_eudkit.RawExpr.bin_op(_BINOP_BITAND, var_expr, mask_expr)
    raise TypeError(f"Expected int, Var, MaskedVar, or Expr, got {type(val).__name__}")


def _make_binop_expr(op: int, lhs: ExprLike, rhs: ExprLike) -> Expr:
    return Expr(
        lambda s: _rust_eudkit.RawExpr.bin_op(
            op, _coerce_raw_expr(lhs, s), _coerce_raw_expr(rhs, s)
        )
    )


def _make_expr_cmp(lhs: ExprLike, comparison: int, rhs: ExprLike) -> Condition:
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.expr_cmp(
                _coerce_raw_expr(lhs, s),
                comparison,
                _coerce_raw_expr(rhs, s),
            )
        ]
    )


def _make_expr_ne(lhs: ExprLike, rhs: ExprLike) -> Condition:
    if isinstance(rhs, int):
        u = int(rhs) & 0xFFFFFFFF
        if u == 0:
            return _make_expr_cmp(lhs, AtLeast, 1)
        if u == 0xFFFFFFFF:
            return _make_expr_cmp(lhs, AtMost, 0xFFFFFFFE)
    diff = _make_binop_expr(_BINOP_SUB, lhs, rhs)
    return _make_expr_cmp(diff, AtLeast, 1)


def _make_expr_lt(lhs: ExprLike, rhs: ExprLike) -> Condition:
    if isinstance(rhs, int):
        val = int(rhs)
        if val <= 0:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return _make_expr_cmp(lhs, AtMost, val - 1)
    if isinstance(lhs, int):
        val = int(lhs) & 0xFFFFFFFF
        if val >= 0xFFFFFFFF:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return _make_expr_cmp(rhs, AtLeast, val + 1)
    sat_diff = _make_binop_expr(_BINOP_SUBSAT, rhs, lhs)
    return _make_expr_cmp(sat_diff, AtLeast, 1)


def _make_expr_gt(lhs: ExprLike, rhs: ExprLike) -> Condition:
    if isinstance(rhs, int):
        val = int(rhs) & 0xFFFFFFFF
        if val >= 0xFFFFFFFF:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return _make_expr_cmp(lhs, AtLeast, val + 1)
    if isinstance(lhs, int):
        val = int(lhs)
        if val <= 0:
            return Condition.from_raw(_rust_eudkit.RawCondition.never())
        return _make_expr_cmp(rhs, AtMost, val - 1)
    sat_diff = _make_binop_expr(_BINOP_SUBSAT, lhs, rhs)
    return _make_expr_cmp(sat_diff, AtLeast, 1)


class _ExprOpsMixin:
    """Shared arithmetic, bitwise, and comparison operator overloads for `Expr`, `Var`, and `MaskedVar`."""

    def __add__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_ADD, self, other)  # type: ignore[arg-type]

    def __radd__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_ADD, other, self)  # type: ignore[arg-type]

    def __sub__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SUB, self, other)  # type: ignore[arg-type]

    def __rsub__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SUB, other, self)  # type: ignore[arg-type]

    def __mul__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_MUL, self, other)  # type: ignore[arg-type]

    def __rmul__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_MUL, other, self)  # type: ignore[arg-type]

    def __floordiv__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_DIV, self, other)  # type: ignore[arg-type]

    def __rfloordiv__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_DIV, other, self)  # type: ignore[arg-type]

    def __mod__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_REM, self, other)  # type: ignore[arg-type]

    def __rmod__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_REM, other, self)  # type: ignore[arg-type]

    def __and__(self, other: ExprLike) -> Expr | MaskedVar:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITAND, self, other)  # type: ignore[arg-type]

    def __rand__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITAND, other, self)  # type: ignore[arg-type]

    def __or__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITOR, self, other)  # type: ignore[arg-type]

    def __ror__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITOR, other, self)  # type: ignore[arg-type]

    def __xor__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITXOR, self, other)  # type: ignore[arg-type]

    def __rxor__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_BITXOR, other, self)  # type: ignore[arg-type]

    def __lshift__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SHL, self, other)  # type: ignore[arg-type]

    def __rlshift__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SHL, other, self)  # type: ignore[arg-type]

    def __rshift__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SHR, self, other)  # type: ignore[arg-type]

    def __rrshift__(self, other: ExprLike) -> Expr:
        if not _is_expr_like(other):
            return NotImplemented
        return _make_binop_expr(_BINOP_SHR, other, self)  # type: ignore[arg-type]

    def __neg__(self) -> Expr:
        return Expr(
            lambda s: _rust_eudkit.RawExpr.unary_neg(_coerce_raw_expr(self, s))  # type: ignore[arg-type]
        )

    def __pos__(self) -> Expr:
        return Expr(lambda s: _coerce_raw_expr(self, s))  # type: ignore[arg-type]

    def __invert__(self) -> Expr:
        return Expr(
            lambda s: _rust_eudkit.RawExpr.unary_not(_coerce_raw_expr(self, s))  # type: ignore[arg-type]
        )


class Expr(_ExprOpsMixin):
    """Represents a compound arithmetic or bitwise expression tree (`HirExpr`)."""

    def __init__(
        self, builder: Callable[[Session], _rust_eudkit.RawExpr]
    ) -> None:
        self._builder = builder

    def resolve(self, session: Session) -> _rust_eudkit.RawExpr:
        return self._builder(session)

    def __eq__(self, other: object) -> Condition:  # type: ignore[override]
        if not _is_expr_like(other):
            raise TypeError("Expression comparison requires an int, Var, MaskedVar, or Expr")
        return _make_expr_cmp(self, Exactly, other)  # type: ignore[arg-type]

    def __ne__(self, other: object) -> Condition:  # type: ignore[override]
        if not _is_expr_like(other):
            raise TypeError("Expression comparison requires an int, Var, MaskedVar, or Expr")
        return _make_expr_ne(self, other)  # type: ignore[arg-type]

    def __ge__(self, other: ExprLike) -> Condition:
        return _make_expr_cmp(self, AtLeast, other)

    def __le__(self, other: ExprLike) -> Condition:
        return _make_expr_cmp(self, AtMost, other)

    def __gt__(self, other: ExprLike) -> Condition:
        return _make_expr_gt(self, other)

    def __lt__(self, other: ExprLike) -> Condition:
        return _make_expr_lt(self, other)


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


class MaskedVar(_ExprOpsMixin):
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
        if isinstance(other, int):
            return self._cmp(Exactly, other)
        if _is_expr_like(other):
            return _make_expr_cmp(self, Exactly, other)  # type: ignore[arg-type]
        raise TypeError("Variable comparison requires an int, Var, MaskedVar, or Expr")

    def __ne__(self, other: object) -> Condition:  # type: ignore[override]
        if isinstance(other, int):
            if (other & 0xFFFFFFFF) == 0:
                return self._cmp(AtLeast, 1)
            if (other & 0xFFFFFFFF) == 0xFFFFFFFF:
                return self._cmp(AtMost, 0xFFFFFFFE)
            return _make_expr_ne(self, other)
        if _is_expr_like(other):
            return _make_expr_ne(self, other)  # type: ignore[arg-type]
        raise TypeError("Variable comparison requires an int, Var, MaskedVar, or Expr")

    def __ge__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            return self._cmp(AtLeast, other)
        return _make_expr_cmp(self, AtLeast, other)

    def __le__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            return self._cmp(AtMost, other)
        return _make_expr_cmp(self, AtMost, other)

    def __gt__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            if (int(other) & 0xFFFFFFFF) >= 0xFFFFFFFF:
                return Condition.from_raw(_rust_eudkit.RawCondition.never())
            return self._cmp(AtLeast, int(other) + 1)
        return _make_expr_gt(self, other)

    def __lt__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            val = int(other)
            if val <= 0:
                return Condition.from_raw(_rust_eudkit.RawCondition.never())
            return self._cmp(AtMost, val - 1)
        return _make_expr_lt(self, other)

    def set(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, SetTo, session=session)

    def add(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, Add, session=session)

    def sub(self, value: int, session: Session | None = None) -> None:
        self.var.set_mask(value, self.mask, Subtract, session=session)


class Var(_ExprOpsMixin):
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

    def __and__(self, mask: ExprLike) -> MaskedVar | Expr:
        if isinstance(mask, int):
            return MaskedVar(self, mask)
        if _is_expr_like(mask):
            return _make_binop_expr(_BINOP_BITAND, self, mask)
        return NotImplemented

    def __eq__(self, other: object) -> Condition:  # type: ignore[override]
        if isinstance(other, int):
            return self._cmp(Exactly, other)
        if _is_expr_like(other):
            return _make_expr_cmp(self, Exactly, other)  # type: ignore[arg-type]
        raise TypeError("Variable comparison requires an int, Var, MaskedVar, or Expr")

    def __ne__(self, other: object) -> Condition:  # type: ignore[override]
        if isinstance(other, int):
            if (other & 0xFFFFFFFF) == 0:
                return self._cmp(AtLeast, 1)
            if (other & 0xFFFFFFFF) == 0xFFFFFFFF:
                return self._cmp(AtMost, 0xFFFFFFFE)
            return _make_expr_ne(self, other)
        if _is_expr_like(other):
            return _make_expr_ne(self, other)  # type: ignore[arg-type]
        raise TypeError("Variable comparison requires an int, Var, MaskedVar, or Expr")

    def __ge__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            return self._cmp(AtLeast, other)
        return _make_expr_cmp(self, AtLeast, other)

    def __le__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            return self._cmp(AtMost, other)
        return _make_expr_cmp(self, AtMost, other)

    def __gt__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            if (int(other) & 0xFFFFFFFF) >= 0xFFFFFFFF:
                return Condition.from_raw(_rust_eudkit.RawCondition.never())
            return self._cmp(AtLeast, int(other) + 1)
        return _make_expr_gt(self, other)

    def __lt__(self, other: ExprLike) -> Condition:
        if isinstance(other, int):
            val = int(other)
            if val <= 0:
                return Condition.from_raw(_rust_eudkit.RawCondition.never())
            return self._cmp(AtMost, val - 1)
        return _make_expr_lt(self, other)

    def set(self, value: ExprLike, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        if isinstance(value, int):
            sess._raw.push_var_assign(vid, _to_i32(value))
        elif _is_expr_like(value):
            sess._raw.push_var_assign_expr(vid, _coerce_raw_expr(value, sess))
        else:
            raise TypeError(f"Unsupported value for Var.set: {value!r}")
        return self

    def assign(self, value: ExprLike, session: Session | None = None) -> Var:
        return self.set(value, session=session)

    def add(self, value: ExprLike, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        if isinstance(value, int):
            val = int(value)
            if val >= 0:
                sess._raw.push_var_add(vid, _to_i32(val))
            else:
                sess._raw.push_var_sub(vid, _to_i32(-val))
            return self
        return self.set(self + value, session=sess)

    def sub(self, value: ExprLike, session: Session | None = None) -> Var:
        sess, vid = self._target_session_and_id(session)
        if isinstance(value, int):
            val = int(value)
            if val >= 0:
                sess._raw.push_var_sub(vid, _to_i32(val))
            else:
                sess._raw.push_var_add(vid, _to_i32(-val))
            return self
        return self.set(self - value, session=sess)

    def __iadd__(self, value: ExprLike) -> Var:
        return self.add(value)

    def __isub__(self, value: ExprLike) -> Var:
        return self.sub(value)

    def __imul__(self, value: ExprLike) -> Var:
        return self.set(self * value)

    def __ifloordiv__(self, value: ExprLike) -> Var:
        return self.set(self // value)

    def __imod__(self, value: ExprLike) -> Var:
        return self.set(self % value)

    def __iand__(self, value: ExprLike) -> Var:
        return self.set(self & value)

    def __ior__(self, value: ExprLike) -> Var:
        return self.set(self | value)

    def __ixor__(self, value: ExprLike) -> Var:
        return self.set(self ^ value)

    def __ilshift__(self, value: ExprLike) -> Var:
        return self.set(self << value)

    def __irshift__(self, value: ExprLike) -> Var:
        return self.set(self >> value)

    def set_mask(
        self,
        value: int,
        mask: int,
        modifier: int = SetTo,
        session: Session | None = None,
    ) -> Var:
        sess, vid = self._target_session_and_id(session)
        sess._raw.push_var_set_mask(
            vid, int(modifier), _to_i32(value), int(mask) & 0xFFFFFFFF
        )
        return self


ExprLike = int | Expr | Var | MaskedVar


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
    player: ExprLike, comparison: int, amount: ExprLike, unit: int | str = 0
) -> Condition:
    if isinstance(player, int) and isinstance(amount, int):
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
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.deaths_dynamic(
                _coerce_raw_expr(player, s),
                int(comparison),
                _coerce_raw_expr(amount, s),
                _resolve_unit(s, unit),
                None,
            )
        ]
    )


def DeathsX(
    player: ExprLike,
    comparison: int,
    amount: ExprLike,
    unit: int | str = 0,
    mask: int = 0xFFFFFFFF,
) -> Condition:
    if isinstance(player, int) and isinstance(amount, int):
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
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.deaths_dynamic(
                _coerce_raw_expr(player, s),
                int(comparison),
                _coerce_raw_expr(amount, s),
                _resolve_unit(s, unit),
                int(mask) & 0xFFFFFFFF,
            )
        ]
    )


def Memory(address: int, comparison: int, value: ExprLike) -> Condition:
    if isinstance(value, int):
        return Condition.from_raw(
            _rust_eudkit.RawCondition.memory(
                int(address) & 0xFFFFFFFF,
                int(comparison),
                int(value) & 0xFFFFFFFF,
            )
        )
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.memory_dynamic(
                int(address) & 0xFFFFFFFF,
                int(comparison),
                _coerce_raw_expr(value, s),
                None,
            )
        ]
    )


def MemoryX(
    address: int, comparison: int, value: ExprLike, mask: int
) -> Condition:
    if isinstance(value, int):
        return Condition.from_raw(
            _rust_eudkit.RawCondition.memory_x(
                int(address) & 0xFFFFFFFF,
                int(comparison),
                int(value) & 0xFFFFFFFF,
                int(mask) & 0xFFFFFFFF,
            )
        )
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.memory_dynamic(
                int(address) & 0xFFFFFFFF,
                int(comparison),
                _coerce_raw_expr(value, s),
                int(mask) & 0xFFFFFFFF,
            )
        ]
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
    player: ExprLike, modifier: int, value: ExprLike, unit: int | str = 0
) -> ActionSpec:
    if isinstance(player, int) and isinstance(value, int):
        return ActionSpec(
            lambda s: _rust_eudkit.RawAction.set_deaths(
                int(player), int(modifier), _to_i32(value), _resolve_unit(s, unit)
            )
        )
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_deaths_dynamic(
            _coerce_raw_expr(player, s),
            int(modifier),
            _coerce_raw_expr(value, s),
            _resolve_unit(s, unit),
            None,
        )
    )


def SetDeathsX(
    player: ExprLike,
    modifier: int,
    value: ExprLike,
    unit: int | str = 0,
    mask: int = 0xFFFFFFFF,
) -> ActionSpec:
    if isinstance(player, int) and isinstance(value, int):
        return ActionSpec(
            lambda s: _rust_eudkit.RawAction.set_deaths_x(
                int(player),
                int(modifier),
                _to_i32(value),
                _resolve_unit(s, unit),
                int(mask) & 0xFFFFFFFF,
            )
        )
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_deaths_dynamic(
            _coerce_raw_expr(player, s),
            int(modifier),
            _coerce_raw_expr(value, s),
            _resolve_unit(s, unit),
            int(mask) & 0xFFFFFFFF,
        )
    )


def SetMemory(address: int, modifier: int, value: ExprLike) -> ActionSpec:
    if isinstance(value, int):
        return ActionSpec(
            lambda _s: _rust_eudkit.RawAction.set_memory(
                int(address) & 0xFFFFFFFF, int(modifier), _to_i32(value)
            )
        )
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_memory_dynamic(
            int(address) & 0xFFFFFFFF,
            int(modifier),
            _coerce_raw_expr(value, s),
            None,
        )
    )


def SetMemoryX(
    address: int, modifier: int, value: ExprLike, mask: int
) -> ActionSpec:
    if isinstance(value, int):
        return ActionSpec(
            lambda _s: _rust_eudkit.RawAction.set_memory_x(
                int(address) & 0xFFFFFFFF,
                int(modifier),
                _to_i32(value),
                int(mask) & 0xFFFFFFFF,
            )
        )
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_memory_dynamic(
            int(address) & 0xFFFFFFFF,
            int(modifier),
            _coerce_raw_expr(value, s),
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
