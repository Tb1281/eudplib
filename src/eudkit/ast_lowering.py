from __future__ import annotations

import ast
import inspect
import textwrap
from collections.abc import Callable
from typing import TYPE_CHECKING, Any

from eudplib.bindings._rust import eudkit as _rust_eudkit

from .types import (
    ActionSpec,
    Condition,
    Expr,
    MaskedVar,
    Var,
    _active_builder_session,
    _to_i32,
    coerce_conditions,
)

if TYPE_CHECKING:
    from .session import Session


class LoopRange:
    """Marker for a runtime `while`-lowered loop range inside `@eud.fn`."""

    def __init__(self, start: int, stop: int | None = None, step: int = 1) -> None:
        if stop is None:
            self.start = 0
            self.stop = int(start)
        else:
            self.start = int(start)
            self.stop = int(stop)
        self.step = int(step)
        if self.step == 0:
            raise ValueError("loop_range step cannot be 0")


def loop_range(start: int, stop: int | None = None, step: int = 1) -> LoopRange:
    return LoopRange(start, stop, step)


class EudFunction:
    """Represents a Python function decorated with `@eud.fn` or `@session.fn`."""

    def __init__(
        self,
        func: Callable[..., Any],
        *,
        session: Session | None = None,
        main_loop: bool = True,
    ) -> None:
        self.func = func
        self.name = func.__name__
        self.default_session = session
        self.is_main_loop = main_loop
        self._fn_node, self._closure_env = _extract_ast_and_env(func)

    def lower_into(self, session: Session, *, main_loop: bool | None = None) -> int:
        is_main = self.is_main_loop if main_loop is None else main_loop
        lowerer = AstToHirLowerer(session, dict(self._closure_env))
        return lowerer.lower_function(self._fn_node, is_main_loop=is_main)

    def inline_into(self, lowerer: AstToHirLowerer) -> None:
        child_env = dict(self._closure_env)
        child_env.update(lowerer.compile_time_vars)
        sub = AstToHirLowerer(
            lowerer.session,
            child_env,
            local_vars=lowerer.local_vars,
        )
        sub.lower_stmt_list(self._fn_node.body)

    def __call__(self, *args: Any, **kwargs: Any) -> Any:
        active = _active_builder_session.get() or self.default_session
        if active is not None:
            return self.lower_into(active, main_loop=self.is_main_loop)
        return self.func(*args, **kwargs)


def _extract_ast_and_env(
    func: Callable[..., Any],
) -> tuple[ast.FunctionDef, dict[str, Any]]:
    src = textwrap.dedent(inspect.getsource(func))
    tree = ast.parse(src)
    fn_node: ast.FunctionDef | None = None
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name == func.__name__:
            fn_node = node
            break
    if fn_node is None:
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef):
                fn_node = node
                break
    if fn_node is None:
        raise ValueError(f"Could not find function definition for {func.__name__}")

    env: dict[str, Any] = {}
    if hasattr(func, "__globals__"):
        env.update(func.__globals__)
    if func.__closure__ and func.__code__.co_freevars:
        for name, cell in zip(func.__code__.co_freevars, func.__closure__):
            try:
                env[name] = cell.cell_contents
            except ValueError:
                pass
    return fn_node, env


class AstToHirLowerer:
    def __init__(
        self,
        session: Session,
        env: dict[str, Any],
        local_vars: dict[str, Var] | None = None,
    ) -> None:
        self.session = session
        self.env = env
        self.local_vars: dict[str, Var] = (
            local_vars if local_vars is not None else {}
        )
        self.compile_time_vars: dict[str, Any] = {}

    def _lookup_symbol(self, name: str) -> Any:
        if name in self.compile_time_vars:
            return self.compile_time_vars[name]
        if name in self.local_vars:
            return self.local_vars[name]
        if name in self.env:
            val = self.env[name]
            if isinstance(val, Var):
                return self._bind_var_to_session(val)
            return val
        if name in self.session._vars_by_name:
            return self.session._vars_by_name[name]
        import builtins

        if hasattr(builtins, name):
            return getattr(builtins, name)
        raise NameError(f"Unknown symbol {name!r} in @eud.fn")

    def _bind_var_to_session(self, var: Var) -> Var:
        if var._session is self.session:
            return var
        if var.name in self.session._vars_by_name:
            return self.session._vars_by_name[var.name]
        return self.session.var(var.init_val, name=var.name)

    def eval_expr(self, node: ast.expr) -> Any:
        if isinstance(node, ast.Constant):
            return node.value
        if isinstance(node, ast.Name):
            return self._lookup_symbol(node.id)
        if isinstance(node, ast.Attribute):
            base = self.eval_expr(node.value)
            return getattr(base, node.attr)
        if isinstance(node, ast.UnaryOp):
            operand = self.eval_expr(node.operand)
            if isinstance(node.op, ast.USub):
                return -operand
            if isinstance(node.op, ast.UAdd):
                return +operand
            if isinstance(node.op, ast.Invert):
                return ~operand
            if isinstance(node.op, ast.Not):
                return not operand
        if isinstance(node, ast.BinOp):
            left = self.eval_expr(node.left)
            right = self.eval_expr(node.right)
            if isinstance(node.op, ast.Add):
                return left + right
            if isinstance(node.op, ast.Sub):
                return left - right
            if isinstance(node.op, ast.Mult):
                return left * right
            if isinstance(node.op, ast.FloorDiv):
                return left // right
            if isinstance(node.op, ast.Mod):
                return left % right
            if isinstance(node.op, ast.BitAnd):
                return left & right
            if isinstance(node.op, ast.BitOr):
                return left | right
            if isinstance(node.op, ast.BitXor):
                return left ^ right
            if isinstance(node.op, ast.LShift):
                return left << right
            if isinstance(node.op, ast.RShift):
                return left >> right
        if isinstance(node, ast.Compare):
            if len(node.ops) != 1 or len(node.comparators) != 1:
                raise ValueError("Chained comparisons are not supported in @eud.fn")
            left = self.eval_expr(node.left)
            right = self.eval_expr(node.comparators[0])
            op = node.ops[0]
            if isinstance(op, ast.Eq):
                return left == right
            if isinstance(op, ast.NotEq):
                return left != right
            if isinstance(op, ast.GtE):
                return left >= right
            if isinstance(op, ast.LtE):
                return left <= right
            if isinstance(op, ast.Gt):
                return left > right
            if isinstance(op, ast.Lt):
                return left < right
        if isinstance(node, ast.Call):
            fn = self.eval_expr(node.func)
            args = [self.eval_expr(a) for a in node.args]
            kwargs = {kw.arg: self.eval_expr(kw.value) for kw in node.keywords if kw.arg}
            # Temporarily clear _active_builder_session so ActionSpec construction does not
            # double-push before lower_stmt handles the returned ActionSpec.
            tok = _active_builder_session.set(None)
            try:
                return fn(*args, **kwargs)
            finally:
                _active_builder_session.reset(tok)
        if isinstance(node, (ast.List, ast.Tuple)):
            return [self.eval_expr(elt) for elt in node.elts]
        raise TypeError(f"Unsupported expression in @eud.fn: {ast.dump(node)}")

    def eval_conditions(self, node: ast.expr) -> list[_rust_eudkit.RawCondition]:
        if isinstance(node, ast.BoolOp) and isinstance(node.op, ast.And):
            out: list[_rust_eudkit.RawCondition] = []
            for v in node.values:
                out.extend(self.eval_conditions(v))
            return out
        val = self.eval_expr(node)
        if isinstance(val, Condition):
            return val.resolve(self.session)
        if isinstance(val, list):
            return coerce_conditions(val, self.session)
        if isinstance(val, bool):
            return [
                _rust_eudkit.RawCondition.always()
                if val
                else _rust_eudkit.RawCondition.never()
            ]
        raise TypeError(f"Expected a Condition in branch/loop test, got {val!r}")

    def lower_function(
        self, fn_node: ast.FunctionDef, *, is_main_loop: bool = True
    ) -> int:
        self.session._raw.begin_function(fn_node.name, is_main_loop)
        try:
            self.lower_stmt_list(fn_node.body)
        except Exception:
            # Clean up any open frames if an error occurs
            raise
        return self.session._raw.end_function()

    def lower_stmt_list(self, stmts: list[ast.stmt]) -> None:
        for stmt in stmts:
            self.lower_stmt(stmt)

    def lower_stmt(self, stmt: ast.stmt) -> None:
        if isinstance(stmt, (ast.Pass, ast.Return)):
            return
        if isinstance(stmt, ast.Break):
            self.session._raw.push_break()
            return
        if isinstance(stmt, ast.Continue):
            self.session._raw.push_continue()
            return
        if isinstance(stmt, ast.Assign):
            self._lower_assign(stmt)
            return
        if isinstance(stmt, ast.AugAssign):
            self._lower_aug_assign(stmt)
            return
        if isinstance(stmt, ast.If):
            self._lower_if(stmt)
            return
        if isinstance(stmt, ast.While):
            self._lower_while(stmt)
            return
        if isinstance(stmt, ast.For):
            self._lower_for(stmt)
            return
        if isinstance(stmt, ast.Expr):
            self._lower_expr_stmt(stmt.value)
            return
        raise TypeError(f"Unsupported statement in @eud.fn: {ast.dump(stmt)}")

    def _resolve_target_var(self, name: str, init_if_new: int = 0) -> Var:
        if name in self.local_vars:
            return self.local_vars[name]
        if name in self.env and isinstance(self.env[name], Var):
            return self._bind_var_to_session(self.env[name])
        if name in self.session._vars_by_name:
            return self.session._vars_by_name[name]
        new_var = self.session.var(init_if_new, name=name)
        self.local_vars[name] = new_var
        return new_var

    def _lower_assign(self, stmt: ast.Assign) -> None:
        if len(stmt.targets) != 1 or not isinstance(stmt.targets[0], ast.Name):
            raise ValueError("Only single variable assignments are supported in @eud.fn")
        target_name = stmt.targets[0].id
        val = self.eval_expr(stmt.value)
        if isinstance(val, int):
            var = self._resolve_target_var(target_name, init_if_new=val)
            self.session._raw.push_var_assign(var.id, _to_i32(val))
            return
        if isinstance(val, (Var, MaskedVar, Expr)):
            var = self._resolve_target_var(target_name, init_if_new=0)
            var.set(val, session=self.session)
            return
        raise TypeError(f"Unsupported assignment value for {target_name!r}: {val!r}")

    def _lower_aug_assign(self, stmt: ast.AugAssign) -> None:
        val = self.eval_expr(stmt.value)
        if isinstance(stmt.target, ast.Name):
            var = self._resolve_target_var(stmt.target.id)
            if isinstance(stmt.op, ast.Add):
                var.add(val, session=self.session)
                return
            if isinstance(stmt.op, ast.Sub):
                var.sub(val, session=self.session)
                return
            if isinstance(stmt.op, ast.Mult):
                var.set(var * val, session=self.session)
                return
            if isinstance(stmt.op, ast.FloorDiv):
                var.set(var // val, session=self.session)
                return
            if isinstance(stmt.op, ast.Mod):
                var.set(var % val, session=self.session)
                return
            if isinstance(stmt.op, ast.BitAnd):
                var.set(var & val, session=self.session)
                return
            if isinstance(stmt.op, ast.BitOr):
                var.set(var | val, session=self.session)
                return
            if isinstance(stmt.op, ast.BitXor):
                var.set(var ^ val, session=self.session)
                return
            if isinstance(stmt.op, ast.LShift):
                var.set(var << val, session=self.session)
                return
            if isinstance(stmt.op, ast.RShift):
                var.set(var >> val, session=self.session)
                return
        else:
            target_obj = self.eval_expr(stmt.target)
            if isinstance(target_obj, MaskedVar) and isinstance(val, int):
                if isinstance(stmt.op, ast.Add):
                    target_obj.add(val, session=self.session)
                    return
                if isinstance(stmt.op, ast.Sub):
                    target_obj.sub(val, session=self.session)
                    return
        raise TypeError(f"Unsupported augmented assignment in @eud.fn: {ast.dump(stmt)}")

    def _lower_expr_stmt(self, expr: ast.expr) -> None:
        if isinstance(expr, ast.Constant) and isinstance(expr.value, str):
            # Docstring
            return
        if isinstance(expr, ast.Call):
            fn = self.eval_expr(expr.func)
            if isinstance(fn, EudFunction):
                fn.inline_into(self)
                return
        tok = _active_builder_session.set(self.session)
        try:
            res = self.eval_expr(expr)
            if isinstance(res, ActionSpec):
                self.session._raw.push_action(res.resolve(self.session))
        finally:
            _active_builder_session.reset(tok)

    def _expand_if_clauses(
        self, node: ast.If
    ) -> tuple[list[tuple[ast.expr, list[ast.stmt]]], list[ast.stmt]]:
        clauses: list[tuple[ast.expr, list[ast.stmt]]] = []
        cur: ast.If | None = node
        final_else: list[ast.stmt] = []

        while cur is not None:
            if isinstance(cur.test, ast.BoolOp) and isinstance(cur.test, ast.BoolOp) and isinstance(cur.test.op, ast.Or):
                for sub_test in cur.test.values:
                    clauses.append((sub_test, cur.body))
            else:
                clauses.append((cur.test, cur.body))

            if len(cur.orelse) == 1 and isinstance(cur.orelse[0], ast.If):
                cur = cur.orelse[0]
            else:
                final_else = cur.orelse
                cur = None

        return clauses, final_else

    def _lower_if(self, node: ast.If) -> None:
        clauses, final_else = self._expand_if_clauses(node)
        first_test, first_body = clauses[0]
        self.session._raw.begin_if(self.eval_conditions(first_test))
        self.lower_stmt_list(first_body)
        self.session._raw.end_if()

        for elif_test, elif_body in clauses[1:]:
            self.session._raw.begin_elif(self.eval_conditions(elif_test))
            self.lower_stmt_list(elif_body)
            self.session._raw.end_elif()

        if final_else:
            self.session._raw.begin_else()
            self.lower_stmt_list(final_else)
            self.session._raw.end_else()

    def _lower_while(self, node: ast.While) -> None:
        conds = self.eval_conditions(node.test)
        self.session._raw.begin_while(conds)
        self.lower_stmt_list(node.body)
        self.session._raw.end_while()

    def _lower_for(self, node: ast.For) -> None:
        if not isinstance(node.target, ast.Name):
            raise ValueError("for loop target in @eud.fn must be a simple variable name")
        target_name = node.target.id
        iter_obj = self.eval_expr(node.iter)

        if isinstance(iter_obj, range):
            if len(iter_obj) <= 1024:
                prev_existed = target_name in self.compile_time_vars
                prev_val = self.compile_time_vars.get(target_name)
                try:
                    for item in iter_obj:
                        self.compile_time_vars[target_name] = item
                        self.lower_stmt_list(node.body)
                finally:
                    if prev_existed:
                        self.compile_time_vars[target_name] = prev_val
                    else:
                        self.compile_time_vars.pop(target_name, None)
                return
            iter_obj = LoopRange(iter_obj.start, iter_obj.stop, iter_obj.step)

        if isinstance(iter_obj, LoopRange):
            loop_var = self._resolve_target_var(target_name, init_if_new=iter_obj.start)
            self.session._raw.push_var_assign(loop_var.id, iter_obj.start)
            cond = (
                (loop_var < iter_obj.stop)
                if iter_obj.step > 0
                else (loop_var > iter_obj.stop)
            )
            self.session._raw.begin_while(cond.resolve(self.session))
            self.lower_stmt_list(node.body)
            loop_var.add(iter_obj.step, session=self.session)
            self.session._raw.end_while()
            return

        raise TypeError(f"Unsupported for-loop iterable in @eud.fn: {iter_obj!r}")
