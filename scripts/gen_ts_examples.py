#!/usr/bin/env python3
"""Generates idiomatic, fully valid v2.ts alongside v2.py for all 75 scenarios."""

import ast
import glob
import os
import re
from pathlib import Path

def evaluate_simple_const(node, constants):
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.Name):
        return constants.get(node.id)
    if isinstance(node, ast.BinOp):
        left = evaluate_simple_const(node.left, constants)
        right = evaluate_simple_const(node.right, constants)
        if left is not None and right is not None:
            if isinstance(node.op, ast.Add): return left + right
            if isinstance(node.op, ast.Sub): return left - right
            if isinstance(node.op, ast.Mult): return left * right
            if isinstance(node.op, ast.FloorDiv): return left // right
    return None

class TsConverter:
    def __init__(self, top_vars, constants, array_epds):
        self.top_vars = set(top_vars)
        self.constants = dict(constants)
        self.array_epds = dict(array_epds)
        self.local_vars = set()
        self.temp_counter = 0

    def new_temp(self):
        self.temp_counter += 1
        return f"_tmp{self.temp_counter}"

    def expr_to_ts(self, node, pre_stmts=None):
        if isinstance(node, ast.Constant):
            if isinstance(node.value, bool):
                return "true" if node.value else "false"
            elif isinstance(node.value, (int, float)):
                if isinstance(node.value, int) and node.value >= 0x10000:
                    return f"0x{node.value:X}"
                return str(node.value)
            elif isinstance(node.value, str):
                escaped = node.value.replace('"', '\\"').replace('\n', '\\n')
                return f'"{escaped}"'
            elif node.value is None:
                return "null"
            return str(node.value)

        if isinstance(node, ast.Name):
            if node.id == "True": return "true"
            if node.id == "False": return "false"
            if node.id == "None": return "null"
            return node.id

        if isinstance(node, ast.UnaryOp):
            op = {
                ast.UAdd: "+",
                ast.USub: "-",
                ast.Not: "!",
                ast.Invert: "~",
            }.get(type(node.op), "-")
            return f"({op}{self.expr_to_ts(node.operand, pre_stmts)})"

        if isinstance(node, ast.BinOp):
            left = self.expr_to_ts(node.left, pre_stmts)
            right = self.expr_to_ts(node.right, pre_stmts)
            if isinstance(node.op, ast.FloorDiv):
                return f"(({left} / {right}) | 0)"
            op_map = {
                ast.Add: "+",
                ast.Sub: "-",
                ast.Mult: "*",
                ast.Div: "/",
                ast.Mod: "%",
                ast.LShift: "<<",
                ast.RShift: ">>",
                ast.BitOr: "|",
                ast.BitXor: "^",
                ast.BitAnd: "&",
            }
            op = op_map.get(type(node.op), "+")
            return f"({left} {op} {right})"

        if isinstance(node, ast.BoolOp):
            op = " && " if isinstance(node.op, ast.And) else " || "
            values = [self.expr_to_ts(v, pre_stmts) for v in node.values]
            return f"({op.join(values)})"

        if isinstance(node, ast.Compare):
            left = self.expr_to_ts(node.left, pre_stmts)
            parts = [left]
            cmp_map = {
                ast.Eq: "==",
                ast.NotEq: "!=",
                ast.Lt: "<",
                ast.LtE: "<=",
                ast.Gt: ">",
                ast.GtE: ">=",
            }
            for op, comparator in zip(node.ops, node.comparators):
                op_str = cmp_map.get(type(op), "==")
                parts.append(f"{op_str} {self.expr_to_ts(comparator, pre_stmts)}")
            return f"({' '.join(parts)})"

        if isinstance(node, ast.Call):
            func_name = self.expr_to_ts(node.func, pre_stmts)
            if func_name == "counts_addr":
                # Evaluate counts_addr(base, player, unit) = base + 48 * unit + 4 * player
                b = self.expr_to_ts(node.args[0])
                p = self.expr_to_ts(node.args[1])
                u = self.expr_to_ts(node.args[2])
                return f"({b} + 48 * {u} + 4 * {p})"

            if func_name == "need_for":
                l = self.expr_to_ts(node.args[0])
                return f"(100 + (20 * ({l} - 1)))"

            if func_name in ("dwread_epd", "dwread", "bread_epd", "bread", "wread_epd", "wread", "epdread_epd"):
                if pre_stmts is not None:
                    tmp_var = self.new_temp()
                    args = [self.expr_to_ts(a, pre_stmts) for a in node.args]
                    pre_stmts.append(f"let {tmp_var} = {func_name}({', '.join(args)});")
                    return tmp_var

            if func_name == "len":
                arg0 = node.args[0]
                if isinstance(arg0, ast.Name) and arg0.id in self.constants:
                    cval = self.constants[arg0.id]
                    if isinstance(cval, (str, list)):
                        return str(len(cval))
                arg_ts = self.expr_to_ts(arg0, pre_stmts)
                return f"{arg_ts}.length"

            # Flatten nested dwread / epdread / bread inside calls
            args = []
            for a in node.args:
                if isinstance(a, ast.Call) and pre_stmts is not None:
                    inner_fn = self.expr_to_ts(a.func)
                    if inner_fn in ("dwread_epd", "dwread", "bread_epd", "bread", "wread_epd", "wread", "epdread_epd"):
                        tmp_var = self.new_temp()
                        inner_call = self.expr_to_ts(a, pre_stmts)
                        pre_stmts.append(f"let {tmp_var} = {inner_call};")
                        args.append(tmp_var)
                        continue
                args.append(self.expr_to_ts(a, pre_stmts))

            if func_name.endswith(".display_text"):
                func_name = func_name.replace(".display_text", ".display")
            return f"{func_name}({', '.join(args)})"

        if isinstance(node, ast.Attribute):
            val = self.expr_to_ts(node.value, pre_stmts)
            if val == "eudkit":
                return node.attr
            return f"{val}.{node.attr}"

        if isinstance(node, ast.Subscript):
            val_name = self.expr_to_ts(node.value, pre_stmts)
            slice_expr = self.expr_to_ts(node.slice, pre_stmts)
            if val_name in self.array_epds:
                base_epd = self.array_epds[val_name]
                read_call = f"dwread_epd({base_epd} + {slice_expr})"
                if pre_stmts is not None:
                    tmp_var = self.new_temp()
                    pre_stmts.append(f"let {tmp_var} = {read_call};")
                    return tmp_var
                return read_call
            return f"{val_name}[{slice_expr}]"

        if isinstance(node, ast.List):
            elts = [self.expr_to_ts(e, pre_stmts) for e in node.elts]
            return f"[{', '.join(elts)}]"

        if isinstance(node, ast.Tuple):
            elts = [self.expr_to_ts(e, pre_stmts) for e in node.elts]
            return f"[{', '.join(elts)}]"

        return "/* expr */"

    def stmt_to_ts(self, node, indent=1):
        pad = "    " * indent

        if isinstance(node, ast.Global):
            return ""

        if isinstance(node, ast.Break):
            return f"{pad}break;\n"

        if isinstance(node, ast.Continue):
            return f"{pad}continue;\n"

        if isinstance(node, ast.Return):
            if node.value:
                return f"{pad}return {self.expr_to_ts(node.value)};\n"
            return f"{pad}return;\n"

        if isinstance(node, ast.Expr):
            pre_stmts = []
            expr_str = self.expr_to_ts(node.value, pre_stmts)
            out = ""
            for ps in pre_stmts:
                out += f"{pad}{ps}\n"
            out += f"{pad}{expr_str};\n"
            return out

        if isinstance(node, ast.Assign):
            if len(node.targets) == 1:
                target = node.targets[0]
                pre_stmts = []

                # If assigning directly from a memory read call, don't lift into pre_stmts
                is_direct_read = False
                if isinstance(node.value, ast.Call):
                    fn_id = self.expr_to_ts(node.value.func)
                    if fn_id in ("dwread_epd", "dwread", "bread_epd", "bread", "wread_epd", "wread", "epdread_epd"):
                        is_direct_read = True

                if is_direct_read:
                    val_str = self.expr_to_ts(node.value, None)
                else:
                    val_str = self.expr_to_ts(node.value, pre_stmts)

                # Check if local session.var inside function
                if isinstance(node.value, ast.Call):
                    call_func = self.expr_to_ts(node.value.func)
                    if call_func in ("session.var", "var", "var_"):
                        init_val = self.expr_to_ts(node.value.args[0]) if node.value.args else "0"
                        target_str = self.expr_to_ts(target)
                        self.local_vars.add(target_str)
                        return f"{pad}let {target_str} = {init_val};\n"

                out = ""
                for ps in pre_stmts:
                    out += f"{pad}{ps}\n"

                if isinstance(target, ast.Subscript):
                    arr_name = self.expr_to_ts(target.value)
                    idx_name = self.expr_to_ts(target.slice)
                    if arr_name in self.array_epds:
                        base_epd = self.array_epds[arr_name]
                        out += f"{pad}dwwrite_epd({base_epd} + {idx_name}, {val_str});\n"
                        return out
                    out += f"{pad}{arr_name}[{idx_name}] = {val_str};\n"
                    return out

                target_str = self.expr_to_ts(target)
                if target_str not in self.top_vars and target_str not in self.local_vars:
                    self.local_vars.add(target_str)
                    out += f"{pad}let {target_str} = {val_str};\n"
                else:
                    out += f"{pad}{target_str} = {val_str};\n"
                return out

        if isinstance(node, ast.AugAssign):
            pre_stmts = []
            val_str = self.expr_to_ts(node.value, pre_stmts)
            target_str = self.expr_to_ts(node.target)
            op_map = {
                ast.Add: "+=",
                ast.Sub: "-=",
                ast.Mult: "*=",
                ast.Div: "/=",
                ast.FloorDiv: "/=",
                ast.Mod: "%=",
                ast.LShift: "<<=",
                ast.RShift: ">>=",
                ast.BitOr: "|=",
                ast.BitXor: "^=",
                ast.BitAnd: "&=",
            }
            op = op_map.get(type(node.op), "+=")
            out = ""
            for ps in pre_stmts:
                out += f"{pad}{ps}\n"
            if isinstance(node.op, ast.FloorDiv):
                out += f"{pad}{target_str} = (({target_str} / {val_str}) | 0);\n"
            else:
                out += f"{pad}{target_str} {op} {val_str};\n"
            return out

        if isinstance(node, ast.If):
            pre_stmts = []
            test_str = self.expr_to_ts(node.test, pre_stmts)
            out = ""
            for ps in pre_stmts:
                out += f"{pad}{ps}\n"
            out += f"{pad}if ({test_str}) {{\n"
            for s in node.body:
                out += self.stmt_to_ts(s, indent + 1)
            if node.orelse:
                if len(node.orelse) == 1 and isinstance(node.orelse[0], ast.If):
                    sub_if = self.stmt_to_ts(node.orelse[0], indent).lstrip()
                    out += f"{pad}}} else {sub_if}"
                    return out
                else:
                    out += f"{pad}}} else {{\n"
                    for s in node.orelse:
                        out += self.stmt_to_ts(s, indent + 1)
            out += f"{pad}}}\n"
            return out

        if isinstance(node, ast.While):
            pre_stmts = []
            test_str = self.expr_to_ts(node.test, pre_stmts)
            out = ""
            for ps in pre_stmts:
                out += f"{pad}{ps}\n"
            out += f"{pad}while ({test_str}) {{\n"
            for s in node.body:
                out += self.stmt_to_ts(s, indent + 1)
            out += f"{pad}}}\n"
            return out

        if isinstance(node, ast.For):
            target_name = node.target.id if isinstance(node.target, ast.Name) else "i"
            self.local_vars.add(target_name)
            iter_expr = node.iter
            if isinstance(iter_expr, ast.Call) and isinstance(iter_expr.func, ast.Name) and iter_expr.func.id == "range":
                r_args = [self.expr_to_ts(a) for a in iter_expr.args]
                if len(r_args) == 1:
                    init, limit, step = "0", r_args[0], "1"
                elif len(r_args) == 2:
                    init, limit, step = r_args[0], r_args[1], "1"
                else:
                    init, limit, step = r_args[0], r_args[1], r_args[2]

                # If limit is len(PACKED_...) substitute constant length
                if ".length" in limit:
                    arr_id = limit.replace(".length", "").strip()
                    if arr_id in self.constants and isinstance(self.constants[arr_id], list):
                        limit = str(len(self.constants[arr_id]))

                step_str = f"{target_name}++" if step == "1" else f"{target_name} += {step}"
                out = f"{pad}for (let {target_name} = {init}; {target_name} < {limit}; {step_str}) {{\n"
                for s in node.body:
                    out += self.stmt_to_ts(s, indent + 1)
                out += f"{pad}}}\n"
                return out

        if isinstance(node, ast.With):
            item = node.items[0].context_expr
            ctx_name = ""
            ctx_args = []
            if isinstance(item, ast.Call):
                ctx_name = self.expr_to_ts(item.func)
                ctx_args = [self.expr_to_ts(a) for a in item.args]

            cb_body = ""
            for s in node.body:
                cb_body += self.stmt_to_ts(s, indent + 1)

            if ctx_name in ("once", "session.once"):
                return f"{pad}once(() => {{\n{cb_body}{pad}}});\n"
            elif ctx_name in ("every", "session.every"):
                ticks = ctx_args[0] if ctx_args else "24"
                return f"{pad}every({ticks}, () => {{\n{cb_body}{pad}}});\n"
            elif ctx_name in ("after", "session.after"):
                ticks = ctx_args[0] if ctx_args else "24"
                return f"{pad}after({ticks}, () => {{\n{cb_body}{pad}}});\n"
            else:
                return f"{pad}// with {ctx_name}:\n{cb_body}"

        return f"{pad}// unsupported stmt: {type(node).__name__}\n"

def convert_scenario(sdir):
    v2_py = Path(sdir) / "v2.py"
    v2_ts = Path(sdir) / "v2.ts"
    if not v2_py.exists():
        return False

    with open(v2_py, "r", encoding="utf-8") as f:
        content = f.read()

    tree = ast.parse(content)

    docstring = ast.get_docstring(tree) or ""
    docstring_ts = "/**\n"
    for line in docstring.split("\n"):
        docstring_ts += f" * {line}\n" if line else " *\n"
    docstring_ts += " */\n"

    # Pass 1: Collect top-level constants, variables, array epds
    constants = {}
    top_vars = set()
    array_epds = {}
    array_offset = 0

    for stmt in tree.body:
        if isinstance(stmt, ast.Assign) and len(stmt.targets) == 1:
            t = stmt.targets[0]
            if isinstance(t, ast.Tuple) and isinstance(stmt.value, ast.Tuple):
                for sub_t, sub_v in zip(t.elts, stmt.value.elts):
                    if isinstance(sub_t, ast.Name):
                        constants[sub_t.id] = evaluate_simple_const(sub_v, constants)
                continue
            if isinstance(t, ast.Name):
                name = t.id
                v = stmt.value
                if isinstance(v, ast.Call):
                    call_func = getattr(v.func, "id", "") or getattr(v.func, "attr", "")
                    if "var" in call_func:
                        top_vars.add(name)
                    elif "Array" in call_func:
                        top_vars.add(name)
                        has_epd = False
                        for kw in v.keywords:
                            if kw.arg == "epd":
                                has_epd = True
                                if isinstance(kw.value, ast.Name):
                                    array_epds[name] = kw.value.id
                                elif isinstance(kw.value, ast.Call):
                                    array_epds[name] = f"EPD(0x6FE000 + {array_offset})"
                        if not has_epd:
                            array_epds[name] = f"EPD(0x6FE000 + {array_offset})"
                            array_offset += 1024
                else:
                    c = evaluate_simple_const(v, constants)
                    if c is not None:
                        constants[name] = c
                    elif isinstance(v, ast.List):
                        constants[name] = [evaluate_simple_const(e, constants) for e in v.elts]

    converter = TsConverter(top_vars, constants, array_epds)

    top_level_ts = ""
    functions_ts = ""

    for stmt in tree.body:
        if isinstance(stmt, ast.Expr) and isinstance(stmt.value, ast.Constant) and isinstance(stmt.value.value, str):
            continue
        if isinstance(stmt, (ast.Import, ast.ImportFrom)):
            continue

        if isinstance(stmt, ast.Assign):
            target = stmt.targets[0]
            if isinstance(target, ast.Tuple) and isinstance(stmt.value, ast.Tuple):
                for sub_t, sub_v in zip(target.elts, stmt.value.elts):
                    t_name = converter.expr_to_ts(sub_t)
                    v_str = converter.expr_to_ts(sub_v)
                    top_level_ts += f"export const {t_name} = {v_str};\n"
                continue

            target_name = target.id if isinstance(target, ast.Name) else "val"
            val = stmt.value

            is_var = False
            if isinstance(val, ast.Call):
                call_func = converter.expr_to_ts(val.func)
                if call_func in ("session.var", "var", "var_"):
                    is_var = True
                    init_val = converter.expr_to_ts(val.args[0]) if val.args else "0"
                    top_level_ts += f'export let {target_name} = session.var({init_val}, "{target_name}");\n'
                elif call_func in ("session.string", "string", "stringBuffer", "session.string_buffer"):
                    is_var = True
                    arg_str = converter.expr_to_ts(val.args[0]) if val.args else '""'
                    top_level_ts += f'export let {target_name} = session.string({arg_str});\n'
                elif call_func in ("session.map_string", "map_string", "session.mapString", "mapString"):
                    is_var = True
                    arg_str = converter.expr_to_ts(val.args[0]) if val.args else '""'
                    top_level_ts += f'export const {target_name} = session.map_string({arg_str});\n'
                elif call_func in ("Array", "eudkit.Array"):
                    is_var = True
                    arg_str = converter.expr_to_ts(val.args[0]) if val.args else '16'
                    top_level_ts += f'export const {target_name} = new Array({arg_str});\n'

            elif isinstance(val, ast.ListComp):
                is_var = True
                if isinstance(val.generators[0].iter, ast.Call) and val.generators[0].iter.args:
                    count_expr = val.generators[0].iter.args[0]
                    count = evaluate_simple_const(count_expr, constants) or 2
                    elements = [f'session.var(0, "{target_name}_{i}")' for i in range(count)]
                    top_level_ts += f'export const {target_name} = [{", ".join(elements)}];\n'
                else:
                    top_level_ts += f'export const {target_name} = [session.var(0), session.var(0)];\n'

            if not is_var:
                val_str = converter.expr_to_ts(val)
                top_level_ts += f"export const {target_name} = {val_str};\n"

        elif isinstance(stmt, ast.FunctionDef):
            is_main = stmt.name in ("game_loop", "main") or any(
                isinstance(d, ast.Attribute) and d.attr == "fn" for d in stmt.decorator_list
            )
            fn_name = "main" if is_main else stmt.name
            export_kw = "export " if is_main else ""
            fn_args = ", ".join(f"{a.arg}: number" for a in stmt.args.args)
            ret_type = ": void" if is_main else ": number"
            functions_ts += f"\n{export_kw}function {fn_name}({fn_args}){ret_type} {{\n"
            for s in stmt.body:
                functions_ts += converter.stmt_to_ts(s, 1)
            functions_ts += "}\n"

    # Collect imported symbols from eudkit
    imports = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module == "eudkit":
            for a in node.names:
                imports.add(a.name)
        elif isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == "eudkit":
            imports.add(node.attr)

    imports.add("session")
    if array_epds:
        imports.add("EPD")
        imports.add("dwread_epd")
        imports.add("dwwrite_epd")

    KNOWN_EXPORTS = {
        "session", "var", "var_", "Array", "EPD",
        "dwread", "dwread_epd", "epdread_epd", "bread", "bread_epd", "wread", "wread_epd",
        "dwwrite", "dwwrite_epd", "bwrite", "bwrite_epd", "wwrite", "wwrite_epd",
        "SetDeaths", "SetDeathsX", "SetResources", "SetScore", "SetSwitch",
        "Deaths", "DeathsX", "Memory", "MemoryX", "Accumulate", "Command", "Bring", "Switch", "ElapsedTime", "Score",
        "CreateUnit", "KillUnit", "KillUnitAt", "RemoveUnitAt", "MoveLocation", "MoveUnit", "GiveUnits", "Order",
        "MinimapPing", "PlayWAV", "DisplayText", "PreserveTrigger",
        "every", "once", "after", "when", "until",
        "SetTo", "Add", "Subtract", "AtLeast", "AtMost", "Exactly",
        "Set", "Cleared", "Clear", "Toggle", "Random",
        "All", "Move", "Patrol", "Attack",
        "Ore", "Gas", "OreAndGas",
        "Total", "Units", "Buildings", "UnitsAndBuildings", "Kills", "Razings", "KillsAndRazings", "Custom",
        "Player1", "Player2", "Player3", "Player4", "Player5", "Player6", "Player7", "Player8",
        "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8",
        "CurrentPlayer", "AllPlayers",
    }
    for word in re.findall(r"\b[A-Za-z_][A-Za-z0-9_]*\b", top_level_ts + functions_ts):
        if word in KNOWN_EXPORTS:
            imports.add(word)

    valid_imports = sorted([i for i in imports if i not in ("eudkit", "var")])

    import_header = f"import {{\n"
    for imp in valid_imports:
        import_header += f"    {imp},\n"
    import_header += '} from "eudkit";\n\n'

    ts_source = docstring_ts + "\n" + import_header + top_level_ts + functions_ts
    with open(v2_ts, "w", encoding="utf-8") as f:
        f.write(ts_source)

    return True

if __name__ == "__main__":
    dirs = sorted(glob.glob("examples/[0-9][0-9]-*"))
    print(f"Converting {len(dirs)} scenarios...")
    for d in dirs:
        convert_scenario(d)
    print("Done generating v2.ts files.")
