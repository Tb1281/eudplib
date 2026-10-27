from __future__ import annotations

import ast
import inspect
import textwrap
from collections.abc import Callable
from typing import TYPE_CHECKING, Any

from eudplib.bindings._rust import eudkit as _rust_eudkit

from .types import (
    ActionSpec,
    AtLeast,
    Condition,
    Exactly,
    Expr,
    MaskedVar,
    Var,
    _active_builder_session,
    _coerce_raw_expr,
    _to_i32,
    bread_epd,
    coerce_conditions,
    dwread_epd,
    epdread_epd,
    wread_epd,
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


class EudFuncPtr:
    """Represents an indirect function pointer (`HirFuncPtr`) in eudkit."""

    def __init__(
        self,
        session: Session,
        ptr_id: int,
        name: str,
        param_count: int,
        return_count: int,
    ) -> None:
        self.session = session
        self.id = ptr_id
        self.name = name
        self.param_count = param_count
        self.return_count = return_count

    def set(self, target: EudFunction | int) -> None:
        if isinstance(target, EudFunction):
            if target.fn_id is None:
                target.lower_into(self.session)
            tgt_id = target.fn_id
        else:
            tgt_id = int(target)
        if tgt_id is None:
            raise ValueError(f"Target function {target} has not been lowered yet")
        self.session._raw.push_func_ptr_assign(self.id, tgt_id)

    def __call__(self, *args: Any) -> Any:
        return self


class EudFunction:
    """Represents a Python function decorated with `@eud.fn` or `@session.fn`."""

    def __init__(
        self,
        func: Callable[..., Any],
        *,
        session: Session | None = None,
        main_loop: bool | None = None,
        inline: bool | None = None,
    ) -> None:
        self.func = func
        self.name = func.__name__
        self.default_session = session
        self.inline = inline
        self._fn_node, self._closure_env = _extract_ast_and_env(func)
        param_count = len(self._fn_node.args.args)
        if main_loop is None:
            self.is_main_loop = (param_count == 0 and inline is None)
        else:
            self.is_main_loop = main_loop
        self.fn_id: int | None = None
        self.params: list[Var] = []
        self.return_count: int = 0

    def lower_into(
        self,
        session: Session,
        *,
        main_loop: bool | None = None,
        inline: bool | None = None,
    ) -> int:
        if self.fn_id is not None and self.default_session is session:
            return self.fn_id
        is_main = self.is_main_loop if main_loop is None else main_loop
        inline_hint = self.inline if inline is None else inline
        lowerer = AstToHirLowerer(session, dict(self._closure_env))
        fn_id = lowerer.lower_function(
            self._fn_node,
            is_main_loop=is_main,
            inline=inline_hint,
        )
        self.fn_id = fn_id
        self.default_session = session
        return fn_id

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
        if active is not None and not self.is_main_loop and self.fn_id is not None:
            return self
        if active is not None:
            return self.lower_into(active, main_loop=self.is_main_loop, inline=self.inline)
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
        if isinstance(node, ast.Subscript):
            base = self.eval_expr(node.value)
            slice_val = self.eval_expr(node.slice)
            return base[slice_val]
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
            return fn(*args, **kwargs)
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
        self,
        fn_node: ast.FunctionDef,
        *,
        is_main_loop: bool = True,
        inline: bool | None = None,
    ) -> int:
        param_vars = []
        for arg in fn_node.args.args:
            var_name = f"{fn_node.name}_{arg.arg}"
            p_var = self.session.var(0, name=var_name)
            self.local_vars[arg.arg] = p_var
            param_vars.append(p_var)

        return_count = 0
        for node in ast.walk(fn_node):
            if isinstance(node, ast.Return) and node.value is not None:
                if isinstance(node.value, (ast.Tuple, ast.List)):
                    return_count = max(return_count, len(node.value.elts))
                else:
                    return_count = max(return_count, 1)

        param_ids = [v.id for v in param_vars]
        self.session._raw.begin_function(
            fn_node.name,
            is_main_loop,
            param_ids,
            return_count,
            inline,
        )
        tok_sess = _active_builder_session.set(self.session)
        try:
            self.lower_stmt_list(fn_node.body)
        finally:
            _active_builder_session.reset(tok_sess)
            fn_id = self.session._raw.end_function()
        return fn_id

    def lower_stmt_list(self, stmts: list[ast.stmt]) -> None:
        for stmt in stmts:
            self.lower_stmt(stmt)

    def lower_stmt(self, stmt: ast.stmt) -> None:
        if isinstance(stmt, ast.Pass):
            return
        if isinstance(stmt, ast.Return):
            self._lower_return(stmt)
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
        if isinstance(stmt, ast.With):
            self._lower_with(stmt)
            return
        if isinstance(stmt, ast.Expr):
            self._lower_expr_stmt(stmt.value)
            return
        raise TypeError(f"Unsupported statement in @eud.fn: {ast.dump(stmt)}")

    def _lower_return(self, stmt: ast.Return) -> None:
        if stmt.value is None:
            return
        if isinstance(stmt.value, (ast.Tuple, ast.List)):
            exprs = [
                _coerce_raw_expr(self.eval_expr(elt), self.session)
                for elt in stmt.value.elts
            ]
            self.session._raw.push_return_values(exprs)
        else:
            expr = _coerce_raw_expr(self.eval_expr(stmt.value), self.session)
            self.session._raw.push_return_values([expr])

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
        if len(stmt.targets) != 1:
            raise ValueError("Only single target assignments are supported in @eud.fn")
        target = stmt.targets[0]

        if isinstance(stmt.value, ast.Call):
            fn = self.eval_expr(stmt.value.func)
            if isinstance(fn, (EudFunction, EudFuncPtr)):
                if isinstance(fn, EudFunction) and fn.fn_id is None:
                    fn.lower_into(self.session)
                raw_args = [
                    _coerce_raw_expr(self.eval_expr(a), self.session)
                    for a in stmt.value.args
                ]
                ret_vars: list[Var] = []
                if isinstance(target, ast.Name):
                    ret_vars.append(self._resolve_target_var(target.id))
                elif isinstance(target, (ast.Tuple, ast.List)):
                    for elt in target.elts:
                        if isinstance(elt, ast.Name):
                            ret_vars.append(self._resolve_target_var(elt.id))
                        else:
                            raise ValueError(f"Unsupported unpack target in @eud.fn: {ast.dump(elt)}")
                else:
                    raise ValueError(f"Unsupported assignment target in @eud.fn: {ast.dump(target)}")

                ret_ids = [v.id for v in ret_vars]
                if isinstance(fn, EudFunction):
                    self.session._raw.push_call(fn.fn_id, raw_args, ret_ids)
                else:
                    self.session._raw.push_call_indirect(fn.id, raw_args, ret_ids)
                return

            if fn in (dwread_epd, epdread_epd, wread_epd, bread_epd) and isinstance(target, ast.Name):
                kw_names = {kw.arg for kw in stmt.value.keywords if kw.arg}
                if "ret" not in kw_names:
                    target_var = self._resolve_target_var(target.id, init_if_new=0)
                    args = [self.eval_expr(a) for a in stmt.value.args]
                    kwargs = {kw.arg: self.eval_expr(kw.value) for kw in stmt.value.keywords if kw.arg}
                    kwargs["ret"] = target_var
                    kwargs["session"] = self.session
                    fn(*args, **kwargs)
                    return

        if isinstance(target, ast.Attribute):
            base = self.eval_expr(target.value)
            val = self.eval_expr(stmt.value)
            setattr(base, target.attr, val)
            return

        if isinstance(target, ast.Subscript):
            base = self.eval_expr(target.value)
            slice_val = self.eval_expr(target.slice)
            val = self.eval_expr(stmt.value)
            base[slice_val] = val
            return

        if not isinstance(target, ast.Name):
            raise ValueError("Only single variable assignments are supported in @eud.fn")
        target_name = target.id
        val = self.eval_expr(stmt.value)
        if isinstance(val, int):
            var = self._resolve_target_var(target_name, init_if_new=val)
            self.session._raw.push_var_assign(var.id, _to_i32(val))
            return
        if isinstance(val, (Var, MaskedVar, Expr)):
            var = self._resolve_target_var(target_name, init_if_new=0)
            var.set(val, session=self.session)
            return
        self.compile_time_vars[target_name] = val
        return

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
        elif isinstance(stmt.target, ast.Attribute):
            base = self.eval_expr(stmt.target.value)
            attr = stmt.target.attr
            cur_val = getattr(base, attr)
            if isinstance(stmt.op, ast.Add):
                setattr(base, attr, cur_val + val)
            elif isinstance(stmt.op, ast.Sub):
                setattr(base, attr, cur_val - val)
            elif isinstance(stmt.op, ast.Mult):
                setattr(base, attr, cur_val * val)
            elif isinstance(stmt.op, ast.BitAnd):
                setattr(base, attr, cur_val & val)
            elif isinstance(stmt.op, ast.BitOr):
                setattr(base, attr, cur_val | val)
            elif isinstance(stmt.op, ast.BitXor):
                setattr(base, attr, cur_val ^ val)
            else:
                raise TypeError(
                    f"Unsupported aug_assign operator on attribute: {ast.dump(stmt.op)}"
                )
            return
        elif isinstance(stmt.target, ast.Subscript):
            base = self.eval_expr(stmt.target.value)
            slice_val = self.eval_expr(stmt.target.slice)
            cur_val = base[slice_val]
            if isinstance(stmt.op, ast.Add):
                base[slice_val] = cur_val + val
            elif isinstance(stmt.op, ast.Sub):
                base[slice_val] = cur_val - val
            elif isinstance(stmt.op, ast.Mult):
                base[slice_val] = cur_val * val
            elif isinstance(stmt.op, ast.BitAnd):
                base[slice_val] = cur_val & val
            elif isinstance(stmt.op, ast.BitOr):
                base[slice_val] = cur_val | val
            elif isinstance(stmt.op, ast.BitXor):
                base[slice_val] = cur_val ^ val
            else:
                raise TypeError(
                    f"Unsupported aug_assign operator on subscript: {ast.dump(stmt.op)}"
                )
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
            if isinstance(fn, (EudFunction, EudFuncPtr)):
                if isinstance(fn, EudFunction) and fn.fn_id is None:
                    fn.lower_into(self.session)
                raw_args = [
                    _coerce_raw_expr(self.eval_expr(a), self.session)
                    for a in expr.args
                ]
                if isinstance(fn, EudFunction):
                    self.session._raw.push_call(fn.fn_id, raw_args, [])
                else:
                    self.session._raw.push_call_indirect(fn.id, raw_args, [])
                return
        res = self.eval_expr(expr)
        if isinstance(res, ActionSpec):
            self.session.do(res)

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

    def _lower_with(self, node: ast.With) -> None:
        for item in node.items:
            # Check if calling once(...) or session.once(...) or bare once
            is_once = False
            once_cond_ast = None
            is_when = False
            when_cond_ast = None
            is_until = False
            until_cond_ast = None
            is_after = False
            after_loops_ast = None
            is_every = False
            every_loops_ast = None

            if isinstance(item.context_expr, ast.Call):
                func = item.context_expr.func
                func_name = getattr(func, "attr", getattr(func, "id", ""))
                if func_name == "once":
                    is_once = True
                    if item.context_expr.args and not (len(item.context_expr.args) == 1 and isinstance(item.context_expr.args[0], ast.Constant) and item.context_expr.args[0].value is None):
                        once_cond_ast = item.context_expr.args[0]
                elif func_name == "when":
                    is_when = True
                    when_cond_ast = item.context_expr.args[0]
                elif func_name == "until":
                    is_until = True
                    until_cond_ast = item.context_expr.args[0]
                elif func_name == "after":
                    is_after = True
                    after_loops_ast = item.context_expr.args[0]
                elif func_name == "every":
                    is_every = True
                    every_loops_ast = item.context_expr.args[0]
            elif isinstance(item.context_expr, ast.Name):
                if item.context_expr.id == "once":
                    is_once = True
            else:
                try:
                    ctx_val = self.eval_expr(item.context_expr)
                    if ctx_val is self.session.once or getattr(ctx_val, "__name__", "") == "once" or "once" in str(type(ctx_val)):
                        is_once = True
                except Exception:
                    pass

            if is_once:
                conds = []
                if once_cond_ast is not None:
                    evaluated_cond = self.eval_expr(once_cond_ast)
                    conds = coerce_conditions(evaluated_cond, self.session)
                self.session._raw.begin_once(conds)
                self.lower_stmt_list(node.body)
                self.session._raw.end_reactive()
                return

            if is_when:
                evaluated_cond = self.eval_expr(when_cond_ast)
                conds = coerce_conditions(evaluated_cond, self.session)
                self.session._raw.begin_when(conds)
                self.lower_stmt_list(node.body)
                self.session._raw.end_reactive()
                return

            if is_until:
                evaluated_cond = self.eval_expr(until_cond_ast)
                conds = coerce_conditions(evaluated_cond, self.session)
                self.session._raw.begin_until(conds)
                self.lower_stmt_list(node.body)
                self.session._raw.end_reactive()
                return

            if is_after:
                loops_val = int(self.eval_expr(after_loops_ast))
                self.session._raw.begin_after(loops_val)
                self.lower_stmt_list(node.body)
                self.session._raw.end_reactive()
                return

            if is_every:
                loops_val = int(self.eval_expr(every_loops_ast))
                self.session._raw.begin_every(loops_val)
                self.lower_stmt_list(node.body)
                self.session._raw.end_reactive()
                return

        raise TypeError(f"Unsupported with-statement in @eud.fn: {ast.dump(node)}")

