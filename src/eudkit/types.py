from __future__ import annotations

import contextvars
from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING

from eudkit import _rust as _rust_eudkit

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
    """Represents a StarCraft trigger action (either emitted via `session.do` or passed to `session.trigger`)."""

    def __init__(
        self, builder: Callable[[Session], _rust_eudkit.RawAction]
    ) -> None:
        self._builder = builder

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


def IsUserCP() -> Condition:
    """Condition that checks whether the local machine player matches CurrentPlayer."""
    return Condition.from_raw(_rust_eudkit.RawCondition.is_user_cp())


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


class MapString:
    """Unified STRx-backed string representation with deduplication and static address/EPD resolution."""

    def __init__(
        self,
        text: str,
        *,
        session: Session | None = None,
    ) -> None:
        self.text = str(text)
        self._session = session or _active_builder_session.get()
        self._raw_info: _rust_eudkit.RawMapString | None = None
        if self._session is not None:
            self._ensure_registered(self._session)

    def _ensure_registered(self, session: Session) -> _rust_eudkit.RawMapString:
        if self._raw_info is None or self._session is not session:
            self._session = session
            self._raw_info = session._raw.register_map_string(self.text)
        return self._raw_info

    @property
    def id(self) -> int:
        sess = self._session or _active_builder_session.get()
        if sess is None:
            raise RuntimeError("MapString requires a Session to resolve string ID")
        return self._ensure_registered(sess).id

    @property
    def offset(self) -> int:
        sess = self._session or _active_builder_session.get()
        if sess is None:
            raise RuntimeError("MapString requires a Session to resolve STRx offset")
        return self._ensure_registered(sess).offset

    @property
    def address(self) -> int:
        sess = self._session or _active_builder_session.get()
        if sess is None:
            raise RuntimeError("MapString requires a Session to resolve virtual address")
        return self._ensure_registered(sess).address

    @property
    def epd(self) -> int:
        sess = self._session or _active_builder_session.get()
        if sess is None:
            raise RuntimeError("MapString requires a Session to resolve EPD")
        return self._ensure_registered(sess).epd

    def display(self, always: bool = True) -> ActionSpec:
        return ActionSpec(
            lambda s: _rust_eudkit.RawAction.display_text_id(
                self._ensure_registered(s).id, bool(always)
            )
        )

    def format(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this MapString buffer."""
        def _build(s: Session) -> _rust_eudkit.RawAction:
            str_id = self._ensure_registered(s).id
            template, raw_args = _build_format_template_and_args(args, s)
            return _rust_eudkit.RawAction.format_map_string(str_id, template or self.text, raw_args)
        return ActionSpec(_build)

    def __repr__(self) -> str:
        if self._raw_info is not None:
            return (
                f"MapString({self.text!r}, id={self.id}, "
                f"addr=0x{self.address:08X}, epd={self.epd})"
            )
        return f"MapString({self.text!r})"


class StringBuffer(MapString):
    """Modern typed string buffer backed by deduplicated STRx storage.

    Supersedes legacy `eudplib` (v1) `StringBuffer` and `DBString`.
    In v1, StringBuffer was an EUDStruct managing dynamic byte copies through runtime
    XOR decryption loops, creating >150KB of trigger bloat.
    In eudkit (v2), StringBuffer is backed by static STRx relocation at `0x191943C8`,
    lowering into residue-stride CPString writes with zero runtime decryption overhead.
    """

    def __init__(
        self,
        content: str | int = 1024,
        *,
        session: Session | None = None,
    ) -> None:
        if isinstance(content, int):
            initial_text = " " * content
        else:
            initial_text = str(content)
        super().__init__(initial_text, session=session)

    def printf(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer."""
        return self.format(*args)

    def insert(self, _index: int, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer (v1 API compatibility)."""
        return self.format(*args)

    def append(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer (v1 API compatibility)."""
        return self.format(*args)

    def print_at(self, line: int, *args: Any) -> ActionSpec:
        """Prints this buffer or formatted expressions to a screen line."""
        if args:
            return print_at(line, *args)
        return print_at(line, self)

    def __repr__(self) -> str:
        if self._raw_info is not None:
            return (
                f"StringBuffer(id={self.id}, addr=0x{self.address:08X}, "
                f"epd={self.epd}, len={len(self.text)})"
            )
        return f"StringBuffer(len={len(self.text)})"


EUDString = StringBuffer
DBString = StringBuffer


def _build_format_template_and_args(
    args: Sequence[Any],
    session: Session,
) -> tuple[str, list[_rust_eudkit.RawExpr]]:
    if not args:
        return "", []
    if (
        len(args) > 1
        and isinstance(args[0], str)
        and (
            "%d" in args[0]
            or "%x" in args[0]
            or "%X" in args[0]
            or "%i" in args[0]
            or "%u" in args[0]
            or "%p" in args[0]
        )
    ):
        template = args[0]
        raw_args = [_coerce_raw_expr(a, session) for a in args[1:]]
        return template, raw_args

    template_parts: list[str] = []
    raw_args: list[_rust_eudkit.RawExpr] = []
    for arg in args:
        if isinstance(arg, str):
            template_parts.append(arg)
        elif isinstance(arg, MapString):
            template_parts.append(arg.text)
        elif _is_expr_like(arg):
            template_parts.append("%d")
            raw_args.append(_coerce_raw_expr(arg, session))
        else:
            template_parts.append(str(arg))
    return "".join(template_parts), raw_args


def f_eprintln(*args: Any, always_display: bool = True) -> ActionSpec:
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.format_display_text(
            *_build_format_template_and_args(args, s),
            always_display=bool(always_display),
        )
    )


def displayText(*args: Any, always_display: bool = True) -> ActionSpec:
    if (
        len(args) == 1
        and isinstance(args[0], str)
        and not (
            "%d" in args[0]
            or "%x" in args[0]
            or "%X" in args[0]
            or "%i" in args[0]
            or "%u" in args[0]
            or "%p" in args[0]
        )
    ):
        return ActionSpec(
            lambda _s: _rust_eudkit.RawAction.display_text(
                str(args[0]), bool(always_display)
            )
        )
    if len(args) == 1 and isinstance(args[0], MapString):
        return args[0].display(always=always_display)
    return f_eprintln(*args, always_display=always_display)


DisplayText = displayText


def print_at(line: int, *args: Any) -> ActionSpec:
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.print_at(
            int(line),
            *_build_format_template_and_args(args, s),
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


# Memory I/O using ReadTable
def dwread_epd(
    epd: ExprLike,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads a 32-bit dword from `epd` using the unified backend ReadTable."""
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(ret, Var):
            sess = ret._session
        elif isinstance(ret, (list, tuple)) and ret and isinstance(ret[0], Var):
            sess = ret[0]._session
        else:
            raise RuntimeError("dwread_epd requires an active Session or Var target")
    dst = ret[0] if isinstance(ret, (list, tuple)) else ret
    if dst is None:
        dst = sess.var(0)
    raw_epd = _coerce_raw_expr(epd, sess)
    sess._raw.push_mem_read(dst.id, raw_epd, 0xFFFFFFFF, 0, 0)
    return dst


def epdread_epd(
    epd: ExprLike,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads a 32-bit pointer from `epd` and converts it to an EPD address using the ReadTable."""
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(ret, Var):
            sess = ret._session
        elif isinstance(ret, (list, tuple)) and ret and isinstance(ret[0], Var):
            sess = ret[0]._session
        else:
            raise RuntimeError("epdread_epd requires an active Session or Var target")
    dst = ret[0] if isinstance(ret, (list, tuple)) else ret
    if dst is None:
        dst = sess.var(0)
    raw_epd = _coerce_raw_expr(epd, sess)
    sess._raw.push_mem_read(dst.id, raw_epd, 0xFFFFFFFC, -2, EPD(0))
    return dst


def wread_epd(
    epd: ExprLike,
    subp: int = 0,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads a 16-bit word at `epd` with subp 0 or 1 using the ReadTable."""
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(ret, Var):
            sess = ret._session
        elif isinstance(ret, (list, tuple)) and ret and isinstance(ret[0], Var):
            sess = ret[0]._session
        else:
            raise RuntimeError("wread_epd requires an active Session or Var target")
    dst = ret[0] if isinstance(ret, (list, tuple)) else ret
    if dst is None:
        dst = sess.var(0)
    raw_epd = _coerce_raw_expr(epd, sess)
    if subp == 0:
        mask, shift = 0x0000FFFF, 0
    elif subp == 1:
        mask, shift = 0xFFFF0000, -16
    else:
        raise ValueError(f"invalid word subp: {subp} (must be 0 or 1)")
    sess._raw.push_mem_read(dst.id, raw_epd, mask, shift, 0)
    return dst


def bread_epd(
    epd: ExprLike,
    subp: int = 0,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads an 8-bit byte at `epd` with subp 0, 1, 2, or 3 using the ReadTable."""
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(ret, Var):
            sess = ret._session
        elif isinstance(ret, (list, tuple)) and ret and isinstance(ret[0], Var):
            sess = ret[0]._session
        else:
            raise RuntimeError("bread_epd requires an active Session or Var target")
    dst = ret[0] if isinstance(ret, (list, tuple)) else ret
    if dst is None:
        dst = sess.var(0)
    raw_epd = _coerce_raw_expr(epd, sess)
    if subp == 0:
        mask, shift = 0x000000FF, 0
    elif subp == 1:
        mask, shift = 0x0000FF00, -8
    elif subp == 2:
        mask, shift = 0x00FF0000, -16
    elif subp == 3:
        mask, shift = 0xFF000000, -24
    else:
        raise ValueError(f"invalid byte subp: {subp} (must be 0, 1, 2, or 3)")
    sess._raw.push_mem_read(dst.id, raw_epd, mask, shift, 0)
    return dst


def dwwrite_epd(
    epd: ExprLike, value: ExprLike, *, session: Session | None = None
) -> None:
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(value, Var):
            sess = value._session
        else:
            raise RuntimeError("dwwrite_epd requires an active Session or Var target")
    act = SetDeaths(epd, SetTo, value, 0)
    sess.do(act)


def wwrite_epd(
    epd: ExprLike, subp: int, value: ExprLike, *, session: Session | None = None
) -> None:
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(value, Var):
            sess = value._session
        else:
            raise RuntimeError("wwrite_epd requires an active Session or Var target")
    mask = 0x0000FFFF if subp == 0 else 0xFFFF0000
    shifted_val = (
        value
        if subp == 0
        else (value << 16 if isinstance(value, int) else (value << 16))
    )
    act = SetDeathsX(epd, SetTo, shifted_val, 0, mask)
    sess.do(act)


def bwrite_epd(
    epd: ExprLike, subp: int, value: ExprLike, *, session: Session | None = None
) -> None:
    sess = session or _active_builder_session.get()
    if sess is None:
        if isinstance(epd, Var):
            sess = epd._session
        elif isinstance(value, Var):
            sess = value._session
        else:
            raise RuntimeError("bwrite_epd requires an active Session or Var target")
    shift = subp * 8
    mask = 0xFF << shift
    shifted_val = (value << shift) if isinstance(value, int) else (value << shift)
    act = SetDeathsX(epd, SetTo, shifted_val, 0, mask)
    sess.do(act)


class Array:
    """EPD-addressed contiguous dword array supporting constant and dynamic indexing."""

    def __init__(
        self,
        length_or_init: int | Sequence[int],
        *,
        epd: ExprLike | None = None,
        session: Session | None = None,
    ) -> None:
        if isinstance(length_or_init, int):
            self.length = length_or_init
            self._init = [0] * self.length
        else:
            self._init = list(length_or_init)
            self.length = len(self._init)
        self._session = session or _active_builder_session.get()
        if epd is not None:
            self._epd = epd
        else:
            self._epd = 0

    @property
    def epd(self) -> ExprLike:
        return self._epd

    def __getitem__(self, key: ExprLike) -> Var:
        target_epd = (
            self._epd if (isinstance(key, int) and key == 0) else self._epd + key
        )
        return dwread_epd(target_epd, session=self._session)

    def __setitem__(self, key: ExprLike, value: ExprLike) -> None:
        target_epd = (
            self._epd if (isinstance(key, int) and key == 0) else self._epd + key
        )
        dwwrite_epd(target_epd, value, session=self._session)


class VArray:
    """Unified 72-byte-stride VProc trigger array supporting constant and dynamic indexing."""

    def __init__(
        self,
        length_or_init: int | Sequence[int],
        *,
        epd: ExprLike | None = None,
        session: Session | None = None,
    ) -> None:
        if isinstance(length_or_init, int):
            self.length = length_or_init
            self._init = [0] * self.length
        else:
            self._init = list(length_or_init)
            self.length = len(self._init)
        self._session = session or _active_builder_session.get()
        self._epd = epd if epd is not None else 0

    @property
    def epd(self) -> ExprLike:
        return self._epd

    def _val_epd(self, key: ExprLike) -> ExprLike:
        # 72-byte stride = 18 dwords per trigger slot.
        # Action 0 player2 (value) field is at offset +87 from trigger base.
        if isinstance(key, int):
            return self._epd + 87 + 18 * key
        else:
            return self._epd + 87 + (key * 18)

    def __getitem__(self, key: ExprLike) -> Var:
        return dwread_epd(self._val_epd(key), session=self._session)

    def __setitem__(self, key: ExprLike, value: ExprLike) -> None:
        dwwrite_epd(self._val_epd(key), value, session=self._session)


_CUNIT_BASE = 0x59CCA8
_CUNIT_STRIDE = 336

_UNSUPPORTED_CUNIT_MEMBERS = {
    "orderQueueHead",
    "order_queue_head",
    "orderQueueTail",
    "order_queue_tail",
    "prevUnitType",
    "prev_unit_type",
    "lastEventTimer",
    "last_event_timer",
    "lastEventColor",
    "last_event_color",
    "loadedUnit1",
    "loaded_unit_1",
    "loadedUnit2",
    "loaded_unit_2",
    "loadedUnit3",
    "loaded_unit_3",
    "loadedUnit4",
    "loaded_unit_4",
    "loadedUnit5",
    "loaded_unit_5",
    "loadedUnit6",
    "loaded_unit_6",
    "loadedUnit7",
    "loaded_unit_7",
    "loadedUnit8",
    "loaded_unit_8",
    "prevBurrowedUnit",
    "prev_burrowed_unit",
    "nextBurrowedUnit",
    "next_burrowed_unit",
    "path",
    "contourBoundsLT",
    "contour_bounds_lt",
    "contourBoundsRB",
    "contour_bounds_rb",
    "contourBoundsL",
    "contour_bounds_l",
    "contourBoundsT",
    "contour_bounds_t",
    "contourBoundsR",
    "contour_bounds_r",
    "contourBoundsB",
    "contour_bounds_b",
    "offsetIndex3by3",
    "offset_index_3by3",
    "unknown0x132",
    "AI",
    "ai",
    "airStrength",
    "air_strength",
    "groundStrength",
    "ground_strength",
    "finderIndexLeft",
    "finder_index_left",
    "finderIndexRight",
    "finder_index_right",
    "finderIndexTop",
    "finder_index_top",
    "finderIndexBottom",
    "finder_index_bottom",
}


class CUnit:
    """Typed handle to a StarCraft: Remastered CUnit structure (0x59CCA8, 336 bytes)."""

    def __init__(
        self,
        epd_or_ptr: ExprLike,
        *,
        session: Session | None = None,
    ) -> None:
        self._session = session or _active_builder_session.get()
        if isinstance(epd_or_ptr, int):
            if epd_or_ptr < 1700:
                self._epd: ExprLike = EPD(_CUNIT_BASE + epd_or_ptr * _CUNIT_STRIDE)
            elif epd_or_ptr >= 0x500000:
                self._epd = EPD(epd_or_ptr)
            else:
                self._epd = epd_or_ptr
        else:
            self._epd = epd_or_ptr

    @property
    def epd(self) -> ExprLike:
        return self._epd

    def __getattr__(self, name: str) -> Any:
        if name in _UNSUPPORTED_CUNIT_MEMBERS:
            raise AttributeError(
                f"StarCraft: Remastered does not support '{name}' and causes in-game errors."
            )
        raise AttributeError(f"'CUnit' object has no attribute '{name}'")

    def __setattr__(self, name: str, value: Any) -> None:
        if name in ("_epd", "_session"):
            super().__setattr__(name, value)
            return
        if name in _UNSUPPORTED_CUNIT_MEMBERS:
            raise AttributeError(
                f"StarCraft: Remastered does not support '{name}' and causes in-game errors."
            )
        prop = getattr(type(self), name, None)
        if isinstance(prop, property) and prop.fset is not None:
            prop.fset(self, value)
            return
        super().__setattr__(name, value)

    # Unit HP (offset 0x008, EPD + 2)
    @property
    def hp(self) -> Var:
        return dwread_epd(self._epd + 2, session=self._session)

    @hp.setter
    def hp(self, val: ExprLike) -> None:
        dwwrite_epd(self._epd + 2, val, session=self._session)

    # Unit Sprite pointer (offset 0x00C, EPD + 3)
    @property
    def sprite(self) -> CSprite:
        s_epd = epdread_epd(self._epd + 3, session=self._session)
        return CSprite(s_epd, session=self._session)

    # Move target unit (offset 0x014, EPD + 5)
    @property
    def move_target(self) -> CUnit:
        t_epd = epdread_epd(self._epd + 5, session=self._session)
        return CUnit(t_epd, session=self._session)

    moveTarget = move_target

    # Unit position (offset 0x028, EPD + 10)
    @property
    def pos(self) -> Var:
        return dwread_epd(self._epd + 10, session=self._session)

    @pos.setter
    def pos(self, val: ExprLike) -> None:
        dwwrite_epd(self._epd + 10, val, session=self._session)

    @property
    def posX(self) -> Var:
        return wread_epd(self._epd + 10, 0, session=self._session)

    @posX.setter
    def posX(self, val: ExprLike) -> None:
        wwrite_epd(self._epd + 10, 0, val, session=self._session)

    pos_x = posX

    @property
    def posY(self) -> Var:
        return wread_epd(self._epd + 10, 1, session=self._session)

    @posY.setter
    def posY(self, val: ExprLike) -> None:
        wwrite_epd(self._epd + 10, 1, val, session=self._session)

    pos_y = posY

    # Current order (offset 0x04D, EPD + 19 subp 1)
    @property
    def order(self) -> Var:
        return bread_epd(self._epd + 19, 1, session=self._session)

    @order.setter
    def order(self, val: ExprLike) -> None:
        bwrite_epd(self._epd + 19, 1, val, session=self._session)

    # Order target position / coordinates (offset 0x058, EPD + 22)
    @property
    def order_coords(self) -> Var:
        return dwread_epd(self._epd + 22, session=self._session)

    @order_coords.setter
    def order_coords(self, val: ExprLike) -> None:
        dwwrite_epd(self._epd + 22, val, session=self._session)

    orderTargetPos = order_coords
    order_target_pos = order_coords

    # Unit status flags (offset 0x0DC, EPD + 55)
    @property
    def status_flags(self) -> Var:
        return dwread_epd(self._epd + 55, session=self._session)

    @status_flags.setter
    def status_flags(self, val: ExprLike) -> None:
        dwwrite_epd(self._epd + 55, val, session=self._session)

    statusFlags = status_flags

    # Status flag helpers
    def set_status_flag(self, value: int, mask: int | None = None) -> None:
        m = value if mask is None else mask
        act = SetDeathsX(self._epd + 55, SetTo, value, 0, m)
        sess = self._session or _active_builder_session.get()
        if sess is not None:
            sess.do(act)

    def clear_status_flag(self, mask: int) -> None:
        self.set_status_flag(0, mask)

    def remove_collision(self) -> None:
        self.set_status_flag(0x00A00000)

    def set_collision(self) -> None:
        self.clear_status_flag(0x00A00000)

    def set_invincible(self) -> None:
        self.set_status_flag(0x04000000)

    def clear_invincible(self) -> None:
        self.clear_status_flag(0x04000000)


_CSPRITE_BASE = 0x629D98
_CSPRITE_STRIDE = 36


class CSprite:
    """Typed handle to a StarCraft: Remastered CSprite structure (0x629D98, 36 bytes)."""

    def __init__(
        self,
        epd_or_ptr: ExprLike,
        *,
        session: Session | None = None,
    ) -> None:
        self._session = session or _active_builder_session.get()
        if isinstance(epd_or_ptr, int):
            if epd_or_ptr < 2500:
                self._epd: ExprLike = EPD(_CSPRITE_BASE + epd_or_ptr * _CSPRITE_STRIDE)
            elif epd_or_ptr >= 0x500000:
                self._epd = EPD(epd_or_ptr)
            else:
                self._epd = epd_or_ptr
        else:
            self._epd = epd_or_ptr

    @property
    def epd(self) -> ExprLike:
        return self._epd

    # Position (offset 0x14, EPD + 5)
    @property
    def pos(self) -> Var:
        return dwread_epd(self._epd + 5, session=self._session)

    @pos.setter
    def pos(self, val: ExprLike) -> None:
        dwwrite_epd(self._epd + 5, val, session=self._session)

    @property
    def posX(self) -> Var:
        return wread_epd(self._epd + 5, 0, session=self._session)

    @posX.setter
    def posX(self, val: ExprLike) -> None:
        wwrite_epd(self._epd + 5, 0, val, session=self._session)

    pos_x = posX

    @property
    def posY(self) -> Var:
        return wread_epd(self._epd + 5, 1, session=self._session)

    @posY.setter
    def posY(self, val: ExprLike) -> None:
        wwrite_epd(self._epd + 5, 1, val, session=self._session)

    pos_y = posY

    # Main graphic image (offset 0x18, EPD + 6)
    @property
    def main_graphic(self) -> Var:
        return dwread_epd(self._epd + 6, session=self._session)

    mainGraphic = main_graphic


class Player:
    """Typed accessor for player properties (minerals, gas, deaths)."""

    def __init__(self, player_id: int, *, session: Session | None = None) -> None:
        self.id = player_id
        self._session = session or _active_builder_session.get()

    @property
    def minerals(self) -> Var:
        return dwread_epd(EPD(0x57F0F0) + self.id, session=self._session)

    @minerals.setter
    def minerals(self, val: ExprLike) -> None:
        dwwrite_epd(EPD(0x57F0F0) + self.id, val, session=self._session)

    ore = minerals

    @property
    def gas(self) -> Var:
        return dwread_epd(EPD(0x57F120) + self.id, session=self._session)

    @gas.setter
    def gas(self, val: ExprLike) -> None:
        dwwrite_epd(EPD(0x57F120) + self.id, val, session=self._session)


class _OnceContextManager:
    """Context manager and callback runner for `with once():` and `once(cb)` blocks.
    
    Guarantees the block executes at most once during the game. If conditions are provided,
    waits until they are satisfied on a trigger loop pass before executing.
    """

    def __init__(
        self,
        conditions: Condition | Sequence[Condition] | None = None,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self.conditions = conditions
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("once(...) callback must be executed inside an active eudkit block")
            with sess.once(conditions):
                callback()

    def __call__(
        self,
        conditions: Any = None,
        callback: Callable[[], Any] | None = None,
    ) -> Any:
        # Support once(callback) or @once
        if callable(conditions) and callback is None and not isinstance(conditions, (Condition, ExprLike)):
            callback = conditions
            conditions = None
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("once(...) callback must be executed inside an active eudkit block")
            with sess.once(conditions):
                return callback()
        return _OnceContextManager(conditions)

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError(
                "with once: can only be used inside an active eudkit function or session block"
            )
        self._cm = sess.once(self.conditions)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


once = _OnceContextManager()


class _WhenContextManager:
    """Context manager and callback runner for `with when(conditions):` and `when(conditions, cb)` blocks.
    
    Standing reactive rule that executes every trigger loop while conditions evaluate to true.
    """

    def __init__(
        self,
        conditions: Condition | Sequence[Condition],
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self.conditions = conditions
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("when(...) callback must be executed inside an active eudkit block")
            with sess.when(conditions):
                callback()

    def __call__(
        self,
        conditions: Condition | Sequence[Condition],
        callback: Callable[[], Any] | None = None,
    ) -> _WhenContextManager:
        return _WhenContextManager(conditions, callback)

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("with when(...) can only be used inside an active eudkit block")
        self._cm = sess.when(self.conditions)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


when = _WhenContextManager


class _UntilContextManager:
    """Context manager and callback runner for `with until(condition):` and `until(condition, cb)` blocks.
    
    Executes every trigger loop until `condition` evaluates to true, at which point
    the block permanently deactivates and never executes again.
    """

    def __init__(
        self,
        condition: Condition,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self.condition = condition
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("until(...) callback must be executed inside an active eudkit block")
            with sess.until(condition):
                callback()

    def __call__(
        self,
        condition: Condition,
        callback: Callable[[], Any] | None = None,
    ) -> _UntilContextManager:
        return _UntilContextManager(condition, callback)

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("with until(...) can only be used inside an active eudkit block")
        self._cm = sess.until(self.condition)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


until = _UntilContextManager


class _AfterContextManager:
    """Context manager and callback runner for `with after(loops):` and `after(loops, cb)` blocks.
    
    Counts down the specified number of StarCraft trigger loops and executes once.
    Avoids native `Wait()` actions which freeze trigger threads and desync multiplayer.
    """

    def __init__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self.loops = loops
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("after(...) callback must be executed inside an active eudkit block")
            with sess.after(loops):
                callback()

    def __call__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> _AfterContextManager:
        return _AfterContextManager(loops, callback)

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("with after(loops) can only be used inside an active eudkit block")
        self._cm = sess.after(self.loops)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


after = _AfterContextManager


class _EveryContextManager:
    """Context manager and callback runner for `with every(loops):` and `every(loops, cb)` blocks.
    
    Periodic clock rule that fires repeatedly every `loops` trigger cycles.
    Deterministic, multiplayer-safe, and independent of fragile engine `Wait()` calls.
    """

    def __init__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self.loops = loops
        if callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError("every(...) callback must be executed inside an active eudkit block")
            with sess.every(loops):
                callback()

    def __call__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> _EveryContextManager:
        return _EveryContextManager(loops, callback)

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            raise RuntimeError("with every(loops) can only be used inside an active eudkit block")
        self._cm = sess.every(self.loops)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


every = _EveryContextManager



