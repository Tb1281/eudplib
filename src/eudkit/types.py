from __future__ import annotations

import contextvars
from collections.abc import Callable, Sequence
from contextlib import AbstractContextManager
from typing import TYPE_CHECKING, NamedTuple

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

# Quantities / Counts
All = 0

# Modifiers
SetTo = 7
Add = 8
Subtract = 9

# Resources
Ore = 0
Gas = 1
OreAndGas = 2

# Switch states & actions
Set = 2
Cleared = 3
Clear = 5
Toggle = 6
Random = 7

# Order types
Move = 0
Patrol = 1
Attack = 2

# Score types
Total = 0
Units = 1
Buildings = 2
UnitsAndBuildings = 3
Kills = 4
Razings = 5
KillsAndRazings = 6
Custom = 7

DEATHS_BASE_ADDR = 0x0058A364


def EPD(address: ExprLike) -> int | Expr:
    """Converts a virtual memory address to its EPD index.

    EPDs index 4-byte dwords relative to the deaths table base ``0x58A364``:
    ``(address - 0x58A364) // 4``. Accepts an integer address or an
    ``Expr``/``Var`` holding a runtime address.
    """
    if isinstance(address, (int, float)):
        return (int(address) - DEATHS_BASE_ADDR) // 4
    return (address - DEATHS_BASE_ADDR) // 4


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
    """Represents one or more conjunctive StarCraft conditions (``AND``).

    Combine conditions with ``&`` to require all of them.
    """

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
    """Represents a StarCraft trigger action.

    The builder is resolved when the action is emitted with ``Session.do`` or
    passed to ``Session.trigger``.
    """

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
    """Condition that is always satisfied (StarCraft "Always")."""
    return Condition.from_raw(_rust_eudkit.RawCondition.always())


def Never() -> Condition:
    """Condition that is never satisfied (StarCraft "Never")."""
    return Condition.from_raw(_rust_eudkit.RawCondition.never())


def IsUserCP() -> Condition:
    """Condition that checks whether the local machine player matches CurrentPlayer."""
    return Condition.from_raw(_rust_eudkit.RawCondition.is_user_cp())


def Deaths(
    player: ExprLike, comparison: int, amount: ExprLike, unit: int | str = 0
) -> Condition:
    """Condition on a player's death counter for one unit type.

    True when ``player`` has suffered ``comparison`` ``amount`` deaths of
    ``unit``. ``comparison`` is ``AtLeast``, ``AtMost``, or ``Exactly``;
    ``unit`` is a unit ID or a map-context unit name (stock ID 0 is Terran
    Marine). ``player`` may be a player ID or an expression evaluating to one.
    """
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
    """Masked variant of :func:`Deaths` using EUD extended death counters.

    Compares the selected bits of the death counter against ``amount``:
    ``(deaths & mask) <comparison> amount``. ``unit`` is a unit ID or a
    map-context unit name.
    """
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
    """Condition on the 32-bit dword at an absolute virtual address.

    True when ``memory[address] <comparison> value``. ``address`` is a raw
    virtual address, not an EPD.
    """
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
    """Masked condition on the dword at an absolute virtual address.

    Compares ``(memory[address] & mask) <comparison> value``. ``address`` is a
    raw virtual address, not an EPD.
    """
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
    """Condition on a player's accumulated resources.

    True when ``player`` has ``comparison`` ``amount`` of ``resource``
    (``Ore``, ``Gas``, or ``OreAndGas``).
    """
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
    """Condition on how many units a player commands.

    True when ``player`` commands ``comparison`` ``amount`` units of type
    ``unit`` (a unit ID or map-context unit name).
    """
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
    """Condition on units a player brings to a location.

    True when ``player`` has ``comparison`` ``amount`` units of type ``unit``
    inside ``location``. Both ``unit`` and ``location`` accept IDs or
    map-context names.
    """
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
    """Condition on a switch's state.

    True when ``switch`` (a switch ID or map-context switch name) is in
    ``state``: ``Set`` or ``Cleared``.
    """
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.switch(
                _resolve_switch(s, switch), int(state)
            )
        ]
    )


def ElapsedTime(comparison: int, seconds: int) -> Condition:
    """Condition on elapsed scenario time.

    True when the game has run ``comparison`` ``seconds`` game seconds since
    the scenario started.
    """
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.elapsed_time(
                int(comparison), int(seconds) & 0xFFFFFFFF
            )
        ]
    )


def Score(
    player: int, score_type: int, comparison: int, amount: int
) -> Condition:
    """Condition on one of a player's score counters.

    True when ``player``'s ``score_type`` counter (``Total``, ``Units``,
    ``Buildings``, ``UnitsAndBuildings``, ``Kills``, ``Razings``,
    ``KillsAndRazings``, or ``Custom``) is ``comparison`` ``amount``.
    """
    return Condition(
        [
            lambda s: _rust_eudkit.RawCondition.score(
                int(player), int(score_type), int(comparison), int(amount) & 0xFFFFFFFF
            )
        ]
    )


# --- Stock Actions ---


def SetDeaths(
    player: ExprLike, modifier: int, value: ExprLike, unit: int | str = 0
) -> ActionSpec:
    """Action that modifies a player's death counter for one unit type.

    Applies ``modifier`` (``SetTo``, ``Add``, or ``Subtract``) with ``value``
    to ``player``'s death counter for ``unit``. ``unit`` is a unit ID or a
    map-context unit name.
    """
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
    """Masked variant of :func:`SetDeaths` using EUD extended death counters.

    Applies ``modifier`` with ``value`` only to the counter bits selected by
    ``mask``; unselected bits are preserved. ``unit`` is a unit ID or a
    map-context unit name.
    """
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
    """Action that modifies the 32-bit dword at an absolute virtual address.

    Applies ``modifier`` (``SetTo``, ``Add``, or ``Subtract``) with ``value``
    to ``memory[address]``. ``address`` is a raw virtual address, not an EPD.
    """
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
    """Masked action that modifies the dword at an absolute virtual address.

    Applies ``modifier`` with ``value`` only to the bits selected by ``mask``;
    ``address`` is a raw virtual address, not an EPD.
    """
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
    """Action that modifies a player's resources.

    Applies ``modifier`` (``SetTo``, ``Add``, or ``Subtract``) with ``amount``
    to ``resource`` (``Ore``, ``Gas``, or ``OreAndGas``).
    """
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

    def display(self, *args: Any, always: bool = True) -> ActionSpec:
        """Displays this static map string to the chat log."""
        if args:
            raise TypeError(
                "MapString is static and cannot be formatted with dynamic arguments. "
                "Use session.string(...) for dynamic formatted string buffers."
            )
        return ActionSpec(
            lambda s: _rust_eudkit.RawAction.display_text_id(
                self._ensure_registered(s).id, bool(always)
            )
        )

    def display_at(self, line: int, *args: Any) -> ActionSpec:
        """Displays this static map string to a fixed screen line (0..12)."""
        if args:
            raise TypeError(
                "MapString is static and cannot be formatted with dynamic arguments. "
                "Use session.string(...) for dynamic formatted string buffers."
            )
        return display_at(line, self.text)

    display_text = display
    Display = display
    print_at = display_at

    def format(self, *args: Any) -> ActionSpec:
        """Restricts formatting on static MapString."""
        raise TypeError(
            "MapString is static and cannot be formatted. "
            "Use session.string(...) to allocate a dynamic formatted string buffer."
        )

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

    def format(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer."""
        def _build(s: Session) -> _rust_eudkit.RawAction:
            str_id = self._ensure_registered(s).id
            if (
                args
                and not isinstance(args[0], str)
                and any(spec in self.text for spec in ("%d", "%x", "%X", "%i", "%u", "%s", "%p"))
            ):
                template = self.text
                raw_args = [_coerce_raw_expr(a, s) for a in args]
            else:
                template, raw_args = _build_format_template_and_args(args, s)
            return _rust_eudkit.RawAction.format_map_string(str_id, template or self.text, raw_args)
        return ActionSpec(_build)

    def display(self, *args: Any, always: bool = True) -> ActionSpec | list[ActionSpec]:
        """Displays this buffer to chat, optionally formatting arguments first."""
        if args:
            return [self.format(*args), super().display(always=always)]
        return super().display(always=always)

    display_text = display
    Display = display

    def printf(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer."""
        return self.format(*args)

    def insert(self, _index: int, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer (v1 API compatibility)."""
        return self.format(*args)

    def append(self, *args: Any) -> ActionSpec:
        """Formats dynamic expressions into this buffer (v1 API compatibility)."""
        return self.format(*args)

    def display_at(self, line: int, *args: Any) -> ActionSpec:
        """Displays this buffer or formatted expressions to a fixed screen line (0..12)."""
        if args:
            if not isinstance(args[0], str) and any(spec in self.text for spec in ("%d", "%x", "%X", "%i", "%u", "%s", "%p")):
                return display_at(line, self.text, *args)
            return display_at(line, *args)
        return display_at(line, self.text)

    print_at = display_at

    def __repr__(self) -> str:
        if self._raw_info is not None:
            return (
                f"StringBuffer(id={self.id}, addr=0x{self.address:08X}, "
                f"epd={self.epd}, len={len(self.text)})"
            )
        return f"StringBuffer(len={len(self.text)})"


EUDString = StringBuffer
DBString = StringBuffer
String = StringBuffer


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
    """Formats values and displays the result in the chat log.

    Accepts a ``%d``/``%x``/``%i``/``%u``/``%p`` style template followed by
    expression arguments, or a sequence of literal strings and expressions that
    are concatenated with ``%d`` placeholders. ``always_display`` sets the
    action's always-display flag.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.format_display_text(
            *_build_format_template_and_args(args, s),
            always_display=bool(always_display),
        )
    )


def displayText(*args: Any, always_display: bool = True) -> ActionSpec:
    """Displays literal text or formatted values in the chat log.

    A single plain string or :class:`MapString` is displayed as-is. Anything
    else is routed through :func:`f_eprintln` for ``%d``-style formatting or
    string/expression concatenation.
    """
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


def display_at(line: int, *args: Any) -> ActionSpec:
    """Displays text or formatted values to a fixed on-screen HUD line (0..12)."""
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.print_at(
            int(line),
            *_build_format_template_and_args(args, s),
        )
    )


DisplayTextAt = display_at
print_at = display_at


def DisplayTextId(string_id: int, always_display: bool = True) -> ActionSpec:
    """Displays a map string by its STRx string ID in the chat log.

    Use this for strings already registered in the map's string table; for new
    text prefer :func:`displayText` or the dynamic string API
    (``session.string``).
    """
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.display_text_id(
            int(string_id), bool(always_display)
        )
    )


def CreateUnit(
    count: int, unit: int | str, location: int | str, player: int
) -> ActionSpec:
    """Creates ``count`` units of a type at a location for a player.

    ``unit`` and ``location`` accept IDs or map-context names. The action has
    no effect while the game is paused.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.create_unit(
            int(count),
            _resolve_unit(s, unit),
            _resolve_location(s, location),
            int(player),
        )
    )


def KillUnitAt(
    count: int, unit: int | str, location: int | str, player: int
) -> ActionSpec:
    """Kills up to ``count`` units of a type at a location for a player.

    ``count`` is ``All`` or a positive number; ``unit`` and ``location`` accept
    IDs or map-context names. Unlike :func:`KillUnit`, this affects only units
    inside ``location`` and only up to ``count`` of them.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.kill_unit_at(
            int(count),
            _resolve_unit(s, unit),
            _resolve_location(s, location),
            int(player),
        )
    )


def MinimapPing(location: int | str) -> ActionSpec:
    """Pings the minimap at ``location`` for all players.

    ``location`` is a location ID or map-context location name.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.minimap_ping(_resolve_location(s, location))
    )


def SetScore(
    player: int, modifier: int, amount: int, score_type: int = Custom
) -> ActionSpec:
    """Action that modifies one of a player's score counters.

    Applies ``modifier`` (``SetTo``, ``Add``, or ``Subtract``) with ``amount``
    to ``score_type`` (``Total``, ``Units``, ``Buildings``, ``Kills``,
    ``Razings``, ``KillsAndRazings``, ``Custom``, ...).
    """
    return ActionSpec(
        lambda _s: _rust_eudkit.RawAction.set_score(
            int(player), int(modifier), int(amount) & 0xFFFFFFFF, int(score_type)
        )
    )


def SetSwitch(switch: int | str, state: int) -> ActionSpec:
    """Action that changes a switch's state.

    ``switch`` is a switch ID or map-context switch name. ``state`` uses the
    switch action constants: ``Set``, ``Clear``, ``Toggle``, or ``Random``.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.set_switch(
            _resolve_switch(s, switch), int(state)
        )
    )


def MoveLocation(
    location: int | str, unit: int | str, player: int, dest_location: int | str
) -> ActionSpec:
    """Centers a location on units of a type owned by a player.

    Moves ``location`` to center on a ``unit`` unit owned by ``player`` that is
    inside the search area ``dest_location``. If no matching unit is found,
    ``location`` moves to the center of ``dest_location``.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.move_location(
            _resolve_location(s, location),
            _resolve_unit(s, unit),
            int(player),
            _resolve_location(s, dest_location),
        )
    )


def RemoveUnitAt(
    count: int, unit: int | str, location: int | str, player: int
) -> ActionSpec:
    """Removes up to ``count`` units of a type at a location for a player.

    Works like :func:`KillUnitAt`, but the units disappear without dying and
    without incrementing death counters. ``count`` is ``All`` or a positive
    number.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.remove_unit_at(
            int(count),
            _resolve_unit(s, unit),
            _resolve_location(s, location),
            int(player),
        )
    )


def KillUnit(unit: int | str, player: int) -> ActionSpec:
    """Kills all units of a type owned by a player.

    The eudkit wrapper order is ``KillUnit(unit, player)``. The underlying
    StarCraft raw trigger action is ``(player, unit, amount)``; ``amount`` is
    unused, and the action affects every unit of ``unit`` owned by ``player``.
    Has no effect while the game is paused.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.kill_unit(
            _resolve_unit(s, unit),
            int(player),
        )
    )


def MoveUnit(
    count: int,
    unit: int | str,
    player: int,
    start_location: int | str,
    dest_location: int | str,
) -> ActionSpec:
    """Teleports up to ``count`` units of a type between locations.

    Moves ``unit`` units owned by ``player`` from ``start_location`` to
    ``dest_location``. ``count`` is ``All`` or a positive number; both
    locations accept IDs or map-context names.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.move_unit(
            int(count),
            _resolve_unit(s, unit),
            int(player),
            _resolve_location(s, start_location),
            _resolve_location(s, dest_location),
        )
    )


def GiveUnits(
    count: int,
    unit: int | str,
    player: int,
    location: int | str,
    new_player: int,
) -> ActionSpec:
    """Transfers units from one player to another.

    Gives up to ``count`` ``unit`` units owned by ``player`` inside ``location``
    to ``new_player``. ``count`` is ``All`` or a positive number.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.give_units(
            int(count),
            _resolve_unit(s, unit),
            int(player),
            _resolve_location(s, location),
            int(new_player),
        )
    )


def _resolve_order(order: int | str) -> int:
    if isinstance(order, int):
        return order
    match str(order).strip().lower():
        case "move":
            return 0
        case "patrol":
            return 1
        case "attack":
            return 2
        case _:
            return int(order)


def Order(
    unit: int | str,
    player: int,
    start_location: int | str,
    order_type: int | str,
    dest_location: int | str,
) -> ActionSpec:
    """Orders units at a location to move, patrol, or attack a destination.

    Applies to ``unit`` units owned by ``player`` inside ``start_location``.
    ``order_type`` accepts ``Move``/``Patrol``/``Attack`` or the strings
    ``"move"``/``"patrol"``/``"attack"``.
    """
    return ActionSpec(
        lambda s: _rust_eudkit.RawAction.order(
            _resolve_unit(s, unit),
            int(player),
            _resolve_location(s, start_location),
            _resolve_order(order_type),
            _resolve_location(s, dest_location),
        )
    )


def PlayWAV(path: str) -> ActionSpec:
    """Plays a WAV sound file bundled with the map.

    ``path`` is the sound file name or relative path understood by StarCraft.
    """
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.play_wav(str(path)))


def Comment(text: str) -> ActionSpec:
    """Adds a comment to the trigger, shown in the trigger editor.

    Comments have no runtime effect on the game.
    """
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.comment(str(text)))


def PreserveTrigger() -> ActionSpec:
    """Action that keeps the enclosing trigger running on later loop passes.

    Without ``PreserveTrigger``, a StarCraft trigger removes itself after it
    fires once.
    """
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.preserve_trigger())


def Victory() -> ActionSpec:
    """Action that ends the scenario in victory for the trigger owner."""
    return ActionSpec(lambda _s: _rust_eudkit.RawAction.victory())


def Defeat() -> ActionSpec:
    """Action that ends the scenario in defeat for the trigger owner."""
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
    elif subp in (1, 2):
        mask, shift = 0xFFFF0000, -16
    else:
        raise ValueError(f"invalid word subp: {subp} (must be 0, 1, or 2)")
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
    """Writes a 32-bit dword at an EPD, using the current session.

    Implemented as a ``SetDeaths(epd, SetTo, value, 0)`` action, so it uses the
    EUD memory write path. ``session`` defaults to the active builder session.
    """
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
    """Writes a 16-bit word at an EPD without disturbing the other half.

    ``subp`` 0 writes the low word (bits 0..15); any other value writes the high
    word (bits 16..31). The write is emitted as a masked ``SetDeathsX`` action.
    """
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
    """Writes an 8-bit byte at an EPD without disturbing the other bytes.

    ``subp`` selects the byte within the dword: 0 is bits 0..7, 1 is bits 8..15,
    2 is bits 16..23, and 3 is bits 24..31. Emitted as a masked ``SetDeathsX``.
    """
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


def dwread(
    address: ExprLike,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads a 32-bit dword from an absolute virtual address.

    Converts ``address`` with :func:`EPD`, then delegates to :func:`dwread_epd`.
    Pass ``ret`` to choose the destination variable.
    """
    return dwread_epd(EPD(address), ret=ret, session=session)


def wread(
    address: int,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads a 16-bit word from an absolute virtual address.

    The half-word is selected from the address alignment (``(address % 4) // 2``)
    and then read through :func:`wread_epd`.
    """
    return wread_epd(EPD(address), (address % 4) // 2, ret=ret, session=session)


def bread(
    address: int,
    *,
    ret: Var | Sequence[Var] | None = None,
    session: Session | None = None,
) -> Var:
    """Reads an 8-bit byte from an absolute virtual address.

    The byte lane is selected from the address alignment (``address % 4``) and
    then read through :func:`bread_epd`.
    """
    return bread_epd(EPD(address), address % 4, ret=ret, session=session)


def dwwrite(
    address: ExprLike,
    value: ExprLike,
    *,
    session: Session | None = None,
) -> None:
    """Writes a 32-bit dword to an absolute virtual address.

    Converts ``address`` with :func:`EPD`, then delegates to
    :func:`dwwrite_epd`.
    """
    dwwrite_epd(EPD(address), value, session=session)


def wwrite(
    address: int,
    value: ExprLike,
    *,
    session: Session | None = None,
) -> None:
    """Writes a 16-bit word to an absolute virtual address.

    Selects the half-word from the address alignment and delegates to
    :func:`wwrite_epd`.
    """
    wwrite_epd(EPD(address), (address % 4) // 2, value, session=session)


def bwrite(
    address: int,
    value: ExprLike,
    *,
    session: Session | None = None,
) -> None:
    """Writes an 8-bit byte to an absolute virtual address.

    Selects the byte lane from the address alignment and delegates to
    :func:`bwrite_epd`.
    """
    bwrite_epd(EPD(address), address % 4, value, session=session)


class Array:
    """EPD-addressed contiguous dword array supporting constant and dynamic indexing.

    Pass a length to allocate zeroed slots, or a sequence of initial dword
    values. ``epd`` optionally supplies the base EPD. Reads go through the
    session ReadTable; writes are emitted as ``SetDeaths`` actions.
    """

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
    """Unified 72-byte-stride VProc trigger array supporting constant and dynamic indexing.

    Pass a length or a sequence of initial values. Indexing reads and writes the
    value field of each 72-byte trigger slot, so dynamic copies remain possible.
    """

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
    """Typed handle to a StarCraft: Remastered CUnit structure (0x59CCA8, 336 bytes).

    ``epd_or_ptr`` accepts a CUnit array index (below 1700), a virtual pointer
    (0x500000 or above), an EPD, or an expression evaluating to one. Array
    indices and pointers are converted to EPD form automatically.
    """

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
    """Typed handle to a StarCraft: Remastered CSprite structure (0x629D98, 36 bytes).

    ``epd_or_ptr`` accepts a sprite array index (below 2500), a virtual pointer
    (0x500000 or above), an EPD, or an expression evaluating to one.
    """

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
    """Typed accessor for player resources (minerals and gas).

    ``player_id`` is the zero-based player slot, so ``Player1`` is 0.
    """

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


class _ReactiveCall(NamedTuple):
    """Canonical argument split for one Reactive Rule builder call."""

    conditions: Any = None
    loops: int | None = None
    callback: Callable[[], Any] | None = None


def _normalize_reactive_call(
    kind: str,
    first: Any = None,
    second: Any = None,
) -> _ReactiveCall:
    """Normalize the overloaded arguments shared by the Reactive Rule builders.

    `once` accepts an optional callback as its only argument; the other rules
    accept either a condition/loop count or that value followed by a callback.
    """
    if second is not None:
        if kind in ("once", "when", "until"):
            return _ReactiveCall(conditions=first, callback=second)
        if kind in ("after", "every"):
            if first is None:
                raise TypeError(f"{kind}() requires a loop count")
            return _ReactiveCall(loops=int(first), callback=second)
    elif kind == "once":
        if callable(first) and not isinstance(first, (Condition, ExprLike)):
            return _ReactiveCall(callback=first)
        return _ReactiveCall(conditions=first)
    elif kind in ("when", "until"):
        return _ReactiveCall(conditions=first)
    elif kind in ("after", "every"):
        if first is None:
            raise TypeError(f"{kind}() requires a loop count")
        return _ReactiveCall(loops=int(first))
    raise TypeError(f"unknown Reactive Rule kind: {kind!r}")


class _ReactiveContextManager:
    """Shared context-manager and callback runner for a Reactive Rule builder."""

    kind: str
    conditions: Any = None
    loops: int | None = None
    callback: Callable[[], Any] | None = None

    def _init_reactive(self, call: _ReactiveCall) -> None:
        self.conditions = call.conditions
        self.loops = call.loops
        self.callback = call.callback
        self._callback_result: Any = None
        if self.callback is not None:
            sess = _active_builder_session.get()
            if sess is None:
                raise RuntimeError(
                    f"{self.kind}(...) callback must be executed inside an active eudkit block"
                )
            with self._session_scope(sess):
                self._callback_result = self.callback()

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        raise NotImplementedError

    def __enter__(self) -> None:
        sess = _active_builder_session.get()
        if sess is None:
            if self.kind == "once":
                raise RuntimeError(
                    "with once: can only be used inside an active eudkit function or session block"
                )
            raise RuntimeError(
                f"with {self.kind}(...) can only be used inside an active eudkit block"
            )
        self._cm = self._session_scope(sess)
        self._cm.__enter__()

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if hasattr(self, "_cm"):
            self._cm.__exit__(exc_type, exc_val, exc_tb)


class _OnceContextManager(_ReactiveContextManager):
    """Context manager and callback runner for `with once():` and `once(cb)` blocks.
    
    Guarantees the block executes at most once during the game. If conditions are provided,
    waits until they are satisfied on a trigger loop pass before executing.
    """

    kind = "once"

    def __init__(
        self,
        conditions: Condition | Sequence[Condition] | None = None,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self._init_reactive(_normalize_reactive_call("once", conditions, callback))

    def __call__(
        self,
        conditions: Any = None,
        callback: Callable[[], Any] | None = None,
    ) -> Any:
        manager = _OnceContextManager(conditions, callback)
        if manager.callback is not None:
            return manager._callback_result
        return manager

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        return sess.once(self.conditions)


once = _OnceContextManager()


class _WhenContextManager(_ReactiveContextManager):
    """Context manager and callback runner for `with when(conditions):` and `when(conditions, cb)` blocks.
    
    Standing reactive rule that executes every trigger loop while conditions evaluate to true.
    """

    kind = "when"

    def __init__(
        self,
        conditions: Condition | Sequence[Condition],
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self._init_reactive(_normalize_reactive_call("when", conditions, callback))

    def __call__(
        self,
        conditions: Condition | Sequence[Condition],
        callback: Callable[[], Any] | None = None,
    ) -> _WhenContextManager:
        return _WhenContextManager(conditions, callback)

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        return sess.when(self.conditions)


when = _WhenContextManager


class _UntilContextManager(_ReactiveContextManager):
    """Context manager and callback runner for `with until(condition):` and `until(condition, cb)` blocks.
    
    Executes every trigger loop until `condition` evaluates to true, at which point
    the block permanently deactivates and never executes again.
    """

    kind = "until"

    def __init__(
        self,
        condition: Condition,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self._init_reactive(_normalize_reactive_call("until", condition, callback))

    def __call__(
        self,
        condition: Condition,
        callback: Callable[[], Any] | None = None,
    ) -> _UntilContextManager:
        return _UntilContextManager(condition, callback)

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        return sess.until(self.conditions)


until = _UntilContextManager


class _AfterContextManager(_ReactiveContextManager):
    """Context manager and callback runner for `with after(loops):` and `after(loops, cb)` blocks.
    
    Counts down the specified number of StarCraft trigger loops and executes once.
    Avoids native `Wait()` actions which freeze trigger threads and desync multiplayer.
    """

    kind = "after"

    def __init__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self._init_reactive(_normalize_reactive_call("after", loops, callback))

    def __call__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> _AfterContextManager:
        return _AfterContextManager(loops, callback)

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        return sess.after(self.loops)


after = _AfterContextManager


class _EveryContextManager(_ReactiveContextManager):
    """Context manager and callback runner for `with every(loops):` and `every(loops, cb)` blocks.
    
    Periodic clock rule that fires repeatedly every `loops` trigger cycles.
    Deterministic, multiplayer-safe, and independent of fragile engine `Wait()` calls.
    """

    kind = "every"

    def __init__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> None:
        self._init_reactive(_normalize_reactive_call("every", loops, callback))

    def __call__(
        self,
        loops: int,
        callback: Callable[[], Any] | None = None,
    ) -> _EveryContextManager:
        return _EveryContextManager(loops, callback)

    def _session_scope(self, sess: Session) -> AbstractContextManager[None]:
        return sess.every(self.loops)


every = _EveryContextManager



