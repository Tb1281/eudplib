use crate::error::{EudkitTsError, Result};
use eudkit_core::canonical::CanonicalSymbols;
use eudkit_core::hir::{
    epd, parse_format_template, CmpOp, Comparison, HirBinOp, HirBuilder, HirExpr, HirModule,
    HirStmt, HirUnaryOp, HirValue, Modifier, PlayerId, PrintTarget, ReactiveRule, ResourceType,
    StockAction, StockCondition, SwitchAction, SwitchState, VarId,
};
use eudkit_core::map_context::MapData;
use eudkit_core::symbols;
use eudkit_core::vocabulary::{frontend_constant, frontend_constants};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::collections::HashMap;

const EPD_BASE_ADDR: i32 = 0x0058_A364;

const MEM_READ_FNS: [&str; 7] = [
    "dwread_epd",
    "dwread",
    "epdread_epd",
    "bread_epd",
    "bread",
    "wread_epd",
    "wread",
];

struct MemReadSpec {
    epd: HirExpr,
    mask: u32,
    initial_val: i32,
}

fn call_callee_name<'a, 'b>(call: &'a CallExpression<'b>) -> Option<&'b str> {
    match &call.callee {
        Expression::Identifier(ident) => Some(ident.name.as_str()),
        Expression::StaticMemberExpression(mem) => match &mem.object {
            Expression::Identifier(obj) if obj.name.as_str() == "eudkit" => {
                Some(mem.property.name.as_str())
            }
            _ => None,
        },
        _ => None,
    }
}

fn address_to_epd(ptr: HirExpr) -> HirExpr {
    HirExpr::BinOp {
        op: HirBinOp::Div,
        lhs: Box::new(HirExpr::BinOp {
            op: HirBinOp::Sub,
            lhs: Box::new(ptr),
            rhs: Box::new(HirExpr::Const(EPD_BASE_ADDR)),
        }),
        rhs: Box::new(HirExpr::Const(4)),
    }
    .fold_constants()
}

pub struct TsCompiler<'a> {
    builder: HirBuilder,
    map_data: &'a MapData,
    canonical_symbols: &'a CanonicalSymbols,
    constants: HashMap<String, i32>,
    array_constants: HashMap<String, Vec<i32>>,
    local_constants: HashMap<String, i32>,
    vars: HashMap<String, VarId>,
    functions: HashMap<String, usize>, // name -> function index
    function_decls: Vec<(String, bool, Vec<VarId>, usize, Option<bool>)>,
    current_fn_idx: Option<usize>,
    loop_vars: HashMap<String, i32>, // compile-time loop counters
    struct_vars: HashMap<String, HirExpr>, // e.g. u -> unit_epd
    string_buffers: HashMap<String, String>, // var name -> template
}

impl<'a> TsCompiler<'a> {
    pub fn new(
        module_name: &str,
        map_data: &'a MapData,
        canonical_symbols: &'a CanonicalSymbols,
    ) -> Self {
        let mut constants = HashMap::new();
        // Register every Frontend spelling from the core vocabulary manifest.
        for constant in frontend_constants() {
            constants.insert(constant.name.to_string(), constant.code);
        }
        // Unit-count pseudo-values are not part of the vocabulary manifest.
        constants.insert("All".to_string(), 0);

        Self {
            builder: HirBuilder::new(module_name),
            map_data,
            canonical_symbols,
            constants,
            array_constants: HashMap::new(),
            local_constants: HashMap::new(),
            vars: HashMap::new(),
            functions: HashMap::new(),
            function_decls: Vec::new(),
            current_fn_idx: None,
            loop_vars: HashMap::new(),
            struct_vars: HashMap::new(),
            string_buffers: HashMap::new(),
        }
    }

    pub fn compile(mut self, program: &Program) -> Result<HirModule> {
        // Pass 1: Collect top-level declarations (vars, consts, functions)
        for stmt in &program.body {
            self.visit_top_level_stmt(stmt)?;
        }

        // Pass 2: Lower function bodies
        for stmt in &program.body {
            self.lower_function_stmt(stmt)?;
        }

        // If no main function exists, ensure main loop exists
        if !self.function_decls.iter().any(|d| d.1) {
            self.builder
                .begin_function("main", true, Vec::new(), 0, Some(false));
            self.builder
                .end_function()
                .map_err(|e| EudkitTsError::Compile(e.to_string()))?;
        }

        self.builder
            .finish()
            .map_err(|e| EudkitTsError::Compile(e.to_string()))
    }

    fn visit_top_level_stmt(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::VariableDeclaration(decl) => {
                self.visit_var_decl(decl)?;
            }
            Statement::ExportDeclaration(decl) => match &decl.declaration {
                Declaration::VariableDeclaration(vdecl) => {
                    self.visit_var_decl(vdecl)?;
                }
                Declaration::FunctionDeclaration(fdecl) => {
                    self.register_function(fdecl, true)?;
                }
                _ => {}
            },
            Statement::FunctionDeclaration(fdecl) => {
                let is_main = fdecl
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str() == "main")
                    .unwrap_or(false);
                self.register_function(fdecl, is_main)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn visit_var_decl(&mut self, decl: &VariableDeclaration) -> Result<()> {
        for declarator in &decl.declarations {
            let BindingPattern::BindingIdentifier(ident) = &declarator.id else {
                continue;
            };
            let var_name = ident.name.as_str();

            if let Some(init) = &declarator.init {
                // Check if init is `var(initVal, name?)` or `var_(...)` or `eud.var(...)`
                if let Expression::CallExpression(call) = init {
                    let is_var_call = match &call.callee {
                        Expression::Identifier(callee_ident) => {
                            callee_ident.name.as_str() == "var"
                                || callee_ident.name.as_str() == "var_"
                        }
                        Expression::StaticMemberExpression(mem) => {
                            mem.property.name.as_str() == "var"
                                || mem.property.name.as_str() == "var_"
                        }
                        _ => false,
                    };

                    if is_var_call {
                        let init_val = if let Some(arg0) =
                            call.arguments.first().and_then(|a| a.as_expression())
                        {
                            self.eval_const_expr(arg0).unwrap_or(0)
                        } else {
                            0
                        };
                        let name = if let Some(arg1) =
                            call.arguments.get(1).and_then(|a| a.as_expression())
                        {
                            if let Expression::StringLiteral(slit) = arg1 {
                                slit.value.as_str().unwrap_or("").to_string()
                            } else {
                                var_name.to_string()
                            }
                        } else {
                            var_name.to_string()
                        };

                        let id = self.builder.add_var(&name, init_val);
                        self.vars.insert(var_name.to_string(), id);
                        continue;
                    }

                    let is_struct_cast = match &call.callee {
                        Expression::StaticMemberExpression(mem) => {
                            if let Expression::Identifier(obj_id) = &mem.object {
                                (obj_id.name.as_str() == "CUnit"
                                    || obj_id.name.as_str() == "CSprite")
                                    && (mem.property.name.as_str() == "cast"
                                        || mem.property.name.as_str() == "fromPtr")
                            } else {
                                false
                            }
                        }
                        Expression::Identifier(ident) => {
                            ident.name.as_str() == "CUnit" || ident.name.as_str() == "CSprite"
                        }
                        _ => false,
                    };

                    if is_struct_cast {
                        if let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) {
                            if let Ok(expr) = self.lower_expr(arg0) {
                                self.struct_vars.insert(var_name.to_string(), expr);
                                continue;
                            }
                        }
                    }

                    let is_string_call = match &call.callee {
                        Expression::Identifier(ident) => {
                            ident.name.as_str() == "string" || ident.name.as_str() == "stringBuffer"
                        }
                        Expression::StaticMemberExpression(mem) => {
                            mem.property.name.as_str() == "string"
                                || mem.property.name.as_str() == "stringBuffer"
                        }
                        _ => false,
                    };

                    if is_string_call {
                        let template = if let Some(arg0) =
                            call.arguments.first().and_then(|a| a.as_expression())
                        {
                            if let Expression::StringLiteral(slit) = arg0 {
                                slit.value.as_str().unwrap_or("").to_string()
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        };
                        self.string_buffers.insert(var_name.to_string(), template);
                        continue;
                    }
                }

                // Check if init is an ArrayExpression (e.g. `[session.var(...), ...]` or `[100, 200, 350]`)
                if let Expression::ArrayExpression(arr) = init {
                    let mut is_var_arr = false;
                    for (idx, el) in arr.elements.iter().enumerate() {
                        if let Some(Expression::CallExpression(call)) = el.as_expression() {
                            let is_var_call = match &call.callee {
                                Expression::Identifier(callee_ident) => {
                                    callee_ident.name.as_str() == "var"
                                        || callee_ident.name.as_str() == "var_"
                                }
                                Expression::StaticMemberExpression(mem) => {
                                    mem.property.name.as_str() == "var"
                                        || mem.property.name.as_str() == "var_"
                                }
                                _ => false,
                            };
                            if is_var_call {
                                is_var_arr = true;
                                let init_val = if let Some(arg0) =
                                    call.arguments.first().and_then(|a| a.as_expression())
                                {
                                    self.eval_const_expr(arg0).unwrap_or(0)
                                } else {
                                    0
                                };
                                let elem_name = if let Some(arg1) =
                                    call.arguments.get(1).and_then(|a| a.as_expression())
                                {
                                    if let Expression::StringLiteral(slit) = arg1 {
                                        slit.value.as_str().unwrap_or("").to_string()
                                    } else {
                                        format!("{var_name}_{idx}")
                                    }
                                } else {
                                    format!("{var_name}_{idx}")
                                };
                                let id = self.builder.add_var(&elem_name, init_val);
                                self.vars.insert(elem_name.clone(), id);
                                self.vars.insert(format!("{var_name}_{idx}"), id);
                            }
                        }
                    }
                    if is_var_arr {
                        continue;
                    }

                    let mut const_vec = Vec::new();
                    let mut all_const = true;
                    for el in &arr.elements {
                        if let Some(e) = el.as_expression() {
                            if let Ok(c) = self.eval_const_expr(e) {
                                const_vec.push(c);
                            } else {
                                all_const = false;
                                break;
                            }
                        } else {
                            all_const = false;
                            break;
                        }
                    }
                    if all_const && !const_vec.is_empty() {
                        self.array_constants.insert(var_name.to_string(), const_vec);
                        continue;
                    }
                }

                // If const declaration with constant expression
                if decl.kind == VariableDeclarationKind::Const {
                    if let Ok(cval) = self.eval_const_expr(init) {
                        self.constants.insert(var_name.to_string(), cval);
                        continue;
                    }
                }

                // Default: regular variable
                if let Ok(cval) = self.eval_const_expr(init) {
                    let id = self.builder.add_var(var_name, cval);
                    self.vars.insert(var_name.to_string(), id);
                } else {
                    let id = self.builder.add_var(var_name, 0);
                    self.vars.insert(var_name.to_string(), id);
                }
            } else {
                let id = self.builder.add_var(var_name, 0);
                self.vars.insert(var_name.to_string(), id);
            }
        }
        Ok(())
    }

    fn register_function(&mut self, fdecl: &Function, is_main: bool) -> Result<()> {
        let name = fdecl
            .id
            .as_ref()
            .map(|id| id.name.as_str())
            .unwrap_or("anon");
        let is_main_loop =
            is_main || name == "main" || name == "onPluginStart" || name == "beforeTriggerExec";
        let return_count = if fdecl.return_type.is_some() { 1 } else { 0 };
        let inline_opt = match name {
            "onPluginStart" | "beforeTriggerExec" | "main" => Some(false),
            _ => None,
        };

        let mut param_vars = Vec::new();
        for param in &fdecl.params.items {
            if let BindingPattern::BindingIdentifier(p_id) = &param.pattern {
                let p_name = p_id.name.as_str();
                let vid = self.builder.add_var(format!("{name}_{p_name}"), 0);
                self.vars.insert(p_name.to_string(), vid);
                param_vars.push(vid);
            }
        }

        let idx = self.function_decls.len();
        self.function_decls.push((
            name.to_string(),
            is_main_loop,
            param_vars,
            return_count,
            inline_opt,
        ));
        self.functions.insert(name.to_string(), idx);
        Ok(())
    }

    fn lower_function_stmt(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::FunctionDeclaration(fdecl) => {
                let name = fdecl
                    .id
                    .as_ref()
                    .map(|id| id.name.as_str())
                    .unwrap_or("anon");
                self.lower_function(name, fdecl)?;
            }
            Statement::ExportDeclaration(decl) => {
                if let Declaration::FunctionDeclaration(fdecl) = &decl.declaration {
                    let name = fdecl
                        .id
                        .as_ref()
                        .map(|id| id.name.as_str())
                        .unwrap_or("anon");
                    self.lower_function(name, fdecl)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_function(&mut self, name: &str, fdecl: &Function) -> Result<()> {
        let Some(&fn_idx) = self.functions.get(name) else {
            return Ok(());
        };
        self.current_fn_idx = Some(fn_idx);

        let (ref fn_name, is_main_loop, ref params, return_count, inline_opt) =
            self.function_decls[fn_idx].clone();
        self.builder.begin_function(
            fn_name,
            is_main_loop,
            params.clone(),
            return_count,
            inline_opt,
        );

        let mut stmts = Vec::new();
        if let Some(body) = &fdecl.body {
            for bstmt in &body.statements {
                self.lower_stmt(bstmt, &mut stmts)?;
            }
        }
        for s in stmts {
            self.builder.push_stmt(s);
        }

        self.builder
            .end_function()
            .map_err(|e| EudkitTsError::Compile(e.to_string()))?;
        self.current_fn_idx = None;
        Ok(())
    }
}

fn split_or_conditions<'e>(expr: &'e Expression<'e>) -> Vec<&'e Expression<'e>> {
    match expr {
        Expression::ParenthesizedExpression(p) => split_or_conditions(&p.expression),
        Expression::LogicalExpression(log) if log.operator == LogicalOperator::Or => {
            let mut left = split_or_conditions(&log.left);
            left.extend(split_or_conditions(&log.right));
            left
        }
        _ => vec![expr],
    }
}

impl<'a> TsCompiler<'a> {
    fn lower_stmt(&mut self, stmt: &Statement, out: &mut Vec<HirStmt>) -> Result<()> {
        match stmt {
            Statement::ExpressionStatement(estmt) => {
                self.lower_expr_stmt(&estmt.expression, out)?;
            }
            Statement::IfStatement(if_stmt) => {
                let branches = split_or_conditions(&if_stmt.test);
                let mut then_body = Vec::new();
                self.lower_block_or_stmt(&if_stmt.consequent, &mut then_body)?;

                let mut else_body = Vec::new();
                if let Some(alt) = &if_stmt.alternate {
                    self.lower_block_or_stmt(alt, &mut else_body)?;
                }

                if branches.len() > 1 {
                    let mut current_else = else_body;
                    for branch in branches.into_iter().rev() {
                        let conds = self.lower_condition(branch)?;
                        current_else = vec![HirStmt::If {
                            conditions: conds,
                            then_body: then_body.clone(),
                            else_body: current_else,
                        }];
                    }
                    out.extend(current_else);
                } else {
                    let conds = self.lower_condition(&if_stmt.test)?;
                    out.push(HirStmt::If {
                        conditions: conds,
                        then_body,
                        else_body,
                    });
                }
            }
            Statement::WhileStatement(while_stmt) => {
                let conds = self.lower_condition(&while_stmt.test)?;
                let mut body = Vec::new();
                self.lower_block_or_stmt(&while_stmt.body, &mut body)?;

                out.push(HirStmt::While {
                    conditions: conds,
                    body,
                });
            }
            Statement::ForStatement(for_stmt) => {
                // Try compile-time loop unrolling: `for (let i = start; i < end; i += step)`
                if let Some(unrolled) = self.try_unroll_for_loop(for_stmt)? {
                    out.extend(unrolled);
                } else {
                    // Fall back to while loop
                    let mut body = Vec::new();
                    self.lower_block_or_stmt(&for_stmt.body, &mut body)?;
                    if let Some(update) = &for_stmt.update {
                        self.lower_expr_stmt(update, &mut body)?;
                    }
                    let conds = if let Some(test) = &for_stmt.test {
                        self.lower_condition(test)?
                    } else {
                        vec![StockCondition::Always]
                    };
                    out.push(HirStmt::While {
                        conditions: conds,
                        body,
                    });
                }
            }
            Statement::SwitchStatement(switch_stmt) => {
                self.lower_switch(switch_stmt, out)?;
            }
            Statement::BreakStatement(_) => {
                out.push(HirStmt::Break);
            }
            Statement::ContinueStatement(_) => {
                out.push(HirStmt::Continue);
            }
            Statement::ReturnStatement(ret_stmt) => {
                if let Some(arg) = &ret_stmt.argument {
                    let expr = self.lower_expr(arg)?;
                    out.push(HirStmt::ReturnValues(vec![expr]));
                }
            }
            Statement::BlockStatement(bstmt) => {
                for s in &bstmt.body {
                    self.lower_stmt(s, out)?;
                }
            }
            Statement::VariableDeclaration(vdecl) => {
                for declarator in &vdecl.declarations {
                    let BindingPattern::BindingIdentifier(ident) = &declarator.id else {
                        continue;
                    };
                    let var_name = ident.name.as_str();
                    if let Some(init) = &declarator.init {
                        let is_struct_cast = match init {
                            Expression::CallExpression(call) => match &call.callee {
                                Expression::StaticMemberExpression(mem) => {
                                    if let Expression::Identifier(obj_id) = &mem.object {
                                        (obj_id.name.as_str() == "CUnit"
                                            || obj_id.name.as_str() == "CSprite")
                                            && (mem.property.name.as_str() == "cast"
                                                || mem.property.name.as_str() == "fromPtr")
                                    } else {
                                        false
                                    }
                                }
                                Expression::Identifier(ident) => {
                                    ident.name.as_str() == "CUnit"
                                        || ident.name.as_str() == "CSprite"
                                }
                                _ => false,
                            },
                            Expression::NewExpression(new_expr) => match &new_expr.callee {
                                Expression::Identifier(ident) => {
                                    ident.name.as_str() == "CUnit"
                                        || ident.name.as_str() == "CSprite"
                                }
                                _ => false,
                            },
                            _ => false,
                        };

                        if is_struct_cast {
                            let arg0_opt = match init {
                                Expression::CallExpression(call) => {
                                    call.arguments.first().and_then(|a| a.as_expression())
                                }
                                Expression::NewExpression(new_expr) => {
                                    new_expr.arguments.first().and_then(|a| a.as_expression())
                                }
                                _ => None,
                            };
                            if let Some(arg0) = arg0_opt {
                                if let Ok(expr) = self.lower_expr(arg0) {
                                    self.struct_vars.insert(var_name.to_string(), expr);
                                    continue;
                                }
                            }
                        }

                        if let Expression::CallExpression(call) = init {
                            if let Some(spec) = self.try_lower_mem_read_call(call)? {
                                let vid = self.builder.add_var(var_name, 0);
                                self.vars.insert(var_name.to_string(), vid);
                                out.push(HirStmt::MemRead {
                                    dst: vid,
                                    epd: spec.epd,
                                    mask: spec.mask,
                                    shift: 0,
                                    initial_val: spec.initial_val,
                                });
                                continue;
                            }
                        }

                        if let Ok(cval) = self.eval_const_expr(init) {
                            self.local_constants.insert(var_name.to_string(), cval);
                        }
                        let expr = self.lower_expr(init)?;
                        let vid = self.builder.add_var(var_name, 0);
                        self.vars.insert(var_name.to_string(), vid);
                        out.push(HirStmt::VarAssignExpr { var: vid, expr });
                    } else {
                        let vid = self.builder.add_var(var_name, 0);
                        self.vars.insert(var_name.to_string(), vid);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_block_or_stmt(&mut self, stmt: &Statement, out: &mut Vec<HirStmt>) -> Result<()> {
        match stmt {
            Statement::BlockStatement(bstmt) => {
                for s in &bstmt.body {
                    self.lower_stmt(s, out)?;
                }
            }
            other => {
                self.lower_stmt(other, out)?;
            }
        }
        Ok(())
    }

    fn try_unroll_for_loop(&mut self, for_stmt: &ForStatement) -> Result<Option<Vec<HirStmt>>> {
        let Some(init) = &for_stmt.init else {
            return Ok(None);
        };
        let Some(test) = &for_stmt.test else {
            return Ok(None);
        };
        let Some(update) = &for_stmt.update else {
            return Ok(None);
        };

        // Parse `let i = 0`
        let ForStatementInit::VariableDeclaration(decl) = init else {
            return Ok(None);
        };
        let Some(declarator) = decl.declarations.first() else {
            return Ok(None);
        };
        let BindingPattern::BindingIdentifier(ident) = &declarator.id else {
            return Ok(None);
        };
        let loop_var_name = ident.name.to_string();
        let Some(init_expr) = &declarator.init else {
            return Ok(None);
        };
        let start_val = self.eval_const_expr(init_expr)?;

        // Parse `i < 40` or `i <= 40`
        let Expression::BinaryExpression(bin_test) = test else {
            return Ok(None);
        };
        let Expression::Identifier(test_ident) = &bin_test.left else {
            return Ok(None);
        };
        if test_ident.name.as_str() != loop_var_name.as_str() {
            return Ok(None);
        }
        let end_val = self.eval_const_expr(&bin_test.right)?;
        let inclusive = match bin_test.operator {
            BinaryOperator::LessThan => false,
            BinaryOperator::LessEqualThan => true,
            _ => return Ok(None),
        };

        // Parse `i++` or `i += 1`
        let step = match update {
            Expression::UpdateExpression(upd) => match upd.operator {
                UpdateOperator::Increment => 1,
                UpdateOperator::Decrement => -1,
            },
            Expression::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Addition =>
            {
                self.eval_const_expr(&assign.right)?
            }
            _ => 1,
        };

        if step == 0 {
            return Ok(None);
        }

        let mut unrolled = Vec::new();
        let mut cur = start_val;
        let limit = if inclusive { end_val + 1 } else { end_val };

        // Safety limit for compile-time unrolling
        let mut iterations = 0;
        while (step > 0 && cur < limit) || (step < 0 && cur > limit) {
            iterations += 1;
            if iterations > 10000 {
                return Err(EudkitTsError::Compile(format!(
                    "loop unroll limit exceeded in for loop for {}",
                    loop_var_name
                )));
            }

            self.loop_vars.insert(loop_var_name.clone(), cur);
            self.lower_block_or_stmt(&for_stmt.body, &mut unrolled)?;
            cur += step;
        }
        self.loop_vars.remove(&loop_var_name);

        Ok(Some(unrolled))
    }

    fn lower_switch(&mut self, switch: &SwitchStatement, out: &mut Vec<HirStmt>) -> Result<()> {
        let disc_expr = self.lower_expr(&switch.discriminant)?;
        let mut cases_reversed = switch.cases.iter().collect::<Vec<_>>();
        cases_reversed.reverse();

        let mut current_else: Vec<HirStmt> = Vec::new();
        for case in cases_reversed {
            let mut case_body = Vec::new();
            for s in &case.consequent {
                // Ignore terminal breaks in switch case
                if matches!(s, Statement::BreakStatement(_)) {
                    continue;
                }
                self.lower_stmt(s, &mut case_body)?;
            }

            if let Some(test) = &case.test {
                let test_expr = self.lower_expr(test)?;
                let cond = match (&disc_expr, &test_expr) {
                    (HirExpr::Var(v), HirExpr::Const(c)) => StockCondition::VarCmp {
                        var: *v,
                        comparison: Comparison::Exactly,
                        value: *c as u32,
                        mask: None,
                    },
                    _ => StockCondition::ExprCmp {
                        lhs: disc_expr.clone(),
                        comparison: Comparison::Exactly,
                        rhs: test_expr,
                    },
                };
                current_else = vec![HirStmt::If {
                    conditions: vec![cond],
                    then_body: case_body,
                    else_body: current_else,
                }];
            } else {
                // Default case
                current_else = case_body;
            }
        }

        out.extend(current_else);
        Ok(())
    }

    fn lower_expr_stmt(&mut self, expr: &Expression, out: &mut Vec<HirStmt>) -> Result<()> {
        match expr {
            Expression::AssignmentExpression(assign) => {
                self.lower_assignment(assign, out)?;
            }
            Expression::UpdateExpression(upd) => {
                self.lower_update(upd, out)?;
            }
            Expression::CallExpression(call) => {
                if let Some(action) = self.try_lower_action(call)? {
                    out.push(HirStmt::Action(action));
                } else if let Some(reactive_stmts) = self.try_lower_reactive_primitive(call)? {
                    out.extend(reactive_stmts);
                } else if let Expression::StaticMemberExpression(mem) = &call.callee {
                    if let Expression::Identifier(obj_ident) = &mem.object {
                        let obj_name = obj_ident.name.as_str();
                        let method_name = mem.property.name.as_str();
                        if let Some(&var_id) = self.vars.get(obj_name) {
                            let arg = call.arguments.first().and_then(|a| a.as_expression());
                            match method_name {
                                "add" => {
                                    if let Some(a) = arg {
                                        if let Ok(c) = self.eval_const_expr(a) {
                                            out.push(HirStmt::VarAddConst {
                                                var: var_id,
                                                value: c,
                                            });
                                        } else {
                                            let rhs = HirExpr::BinOp {
                                                op: HirBinOp::Add,
                                                lhs: Box::new(HirExpr::Var(var_id)),
                                                rhs: Box::new(self.lower_expr(a)?),
                                            }
                                            .fold_constants();
                                            out.push(HirStmt::VarAssignExpr {
                                                var: var_id,
                                                expr: rhs,
                                            });
                                        }
                                    }
                                }
                                "subtract" | "sub" => {
                                    if let Some(a) = arg {
                                        if let Ok(c) = self.eval_const_expr(a) {
                                            out.push(HirStmt::VarSubConst {
                                                var: var_id,
                                                value: c,
                                            });
                                        } else {
                                            let rhs = HirExpr::BinOp {
                                                op: HirBinOp::Sub,
                                                lhs: Box::new(HirExpr::Var(var_id)),
                                                rhs: Box::new(self.lower_expr(a)?),
                                            }
                                            .fold_constants();
                                            out.push(HirStmt::VarAssignExpr {
                                                var: var_id,
                                                expr: rhs,
                                            });
                                        }
                                    }
                                }
                                "setTo" | "set" => {
                                    if let Some(a) = arg {
                                        let mut handled_call = false;
                                        if let Expression::CallExpression(call) = a {
                                            if let Expression::Identifier(ident) = &call.callee {
                                                if let Some(&fn_idx) =
                                                    self.functions.get(ident.name.as_str())
                                                {
                                                    let mut args = Vec::new();
                                                    for arg_item in &call.arguments {
                                                        if let Some(e) = arg_item.as_expression() {
                                                            args.push(self.lower_expr(e)?);
                                                        }
                                                    }
                                                    out.push(HirStmt::Call {
                                                        callee: eudkit_core::hir::FuncId(
                                                            fn_idx as u32,
                                                        ),
                                                        args,
                                                        rets: vec![var_id],
                                                    });
                                                    handled_call = true;
                                                }
                                            }
                                        }
                                        if !handled_call {
                                            if let Ok(c) = self.eval_const_expr(a) {
                                                out.push(HirStmt::VarAssignConst {
                                                    var: var_id,
                                                    value: c,
                                                });
                                            } else {
                                                let rhs = self.lower_expr(a)?;
                                                out.push(HirStmt::VarAssignExpr {
                                                    var: var_id,
                                                    expr: rhs,
                                                });
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                } else if let Expression::Identifier(ident) = &call.callee {
                    if let Some(&fn_idx) = self.functions.get(ident.name.as_str()) {
                        let mut args = Vec::new();
                        for a in &call.arguments {
                            if let Some(e) = a.as_expression() {
                                args.push(self.lower_expr(e)?);
                            }
                        }
                        out.push(HirStmt::Call {
                            callee: eudkit_core::hir::FuncId(fn_idx as u32),
                            args,
                            rets: Vec::new(),
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn try_lower_mem_read_call(&mut self, call: &CallExpression) -> Result<Option<MemReadSpec>> {
        let Some(fname) = call_callee_name(call) else {
            return Ok(None);
        };
        if !MEM_READ_FNS.contains(&fname) {
            return Ok(None);
        }
        let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) else {
            return Ok(None);
        };

        let (epd, mask, initial_val) = match fname {
            "dwread_epd" => (self.lower_expr(arg0)?, 0xFFFF_FFFF, 0),
            "dwread" => (address_to_epd(self.lower_expr(arg0)?), 0xFFFF_FFFF, 0),
            "epdread_epd" => (self.lower_expr(arg0)?, 0xFFFF_FFFF, -(EPD_BASE_ADDR / 4)),
            "bread_epd" => (self.lower_expr(arg0)?, 0xFF, 0),
            "bread" => (address_to_epd(self.lower_expr(arg0)?), 0xFF, 0),
            "wread_epd" => (self.lower_expr(arg0)?, 0xFFFF, 0),
            "wread" => (address_to_epd(self.lower_expr(arg0)?), 0xFFFF, 0),
            _ => return Ok(None),
        };
        Ok(Some(MemReadSpec {
            epd,
            mask,
            initial_val,
        }))
    }

    fn lower_assignment(
        &mut self,
        assign: &AssignmentExpression,
        out: &mut Vec<HirStmt>,
    ) -> Result<()> {
        let resolved_var_id = match &assign.left {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                let var_name = ident.name.as_str();
                self.vars.get(var_name).copied()
            }
            AssignmentTarget::ComputedMemberExpression(comp) => {
                if let Expression::Identifier(ident) = &comp.object {
                    if let Ok(idx) = self.eval_const_expr(&comp.expression) {
                        let var_name = format!("{}_{}", ident.name.as_str(), idx);
                        self.vars.get(&var_name).copied()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(var_id) = resolved_var_id {
            if assign.operator == AssignmentOperator::Assign {
                if let Expression::CallExpression(call) = &assign.right {
                    if let Some(spec) = self.try_lower_mem_read_call(call)? {
                        out.push(HirStmt::MemRead {
                            dst: var_id,
                            epd: spec.epd,
                            mask: spec.mask,
                            shift: 0,
                            initial_val: spec.initial_val,
                        });
                        return Ok(());
                    }

                    if let Some(fname) = call_callee_name(call) {
                        if !MEM_READ_FNS.contains(&fname) {
                            if let Some(&fn_idx) = self.functions.get(fname) {
                                let mut args = Vec::new();
                                for a in &call.arguments {
                                    if let Some(e) = a.as_expression() {
                                        args.push(self.lower_expr(e)?);
                                    }
                                }
                                out.push(HirStmt::Call {
                                    callee: eudkit_core::hir::FuncId(fn_idx as u32),
                                    args,
                                    rets: vec![var_id],
                                });
                                return Ok(());
                            }
                        }
                    }
                }
            }

            let rhs_expr = self.lower_expr(&assign.right)?;
            match assign.operator {
                AssignmentOperator::Assign => {
                    if let HirExpr::Const(c) = rhs_expr {
                        out.push(HirStmt::VarAssignConst {
                            var: var_id,
                            value: c,
                        });
                    } else {
                        out.push(HirStmt::VarAssignExpr {
                            var: var_id,
                            expr: rhs_expr,
                        });
                    }
                }
                AssignmentOperator::Addition => {
                    if let HirExpr::Const(c) = rhs_expr {
                        out.push(HirStmt::VarAddConst {
                            var: var_id,
                            value: c,
                        });
                    } else {
                        let add_expr = HirExpr::BinOp {
                            op: HirBinOp::Add,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr {
                            var: var_id,
                            expr: add_expr,
                        });
                    }
                }
                AssignmentOperator::Subtraction => {
                    if let HirExpr::Const(c) = rhs_expr {
                        out.push(HirStmt::VarSubConst {
                            var: var_id,
                            value: c,
                        });
                    } else {
                        let sub_expr = HirExpr::BinOp {
                            op: HirBinOp::Sub,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr {
                            var: var_id,
                            expr: sub_expr,
                        });
                    }
                }
                _ => {
                    let bin_op = match assign.operator {
                        AssignmentOperator::Multiplication => Some(HirBinOp::Mul),
                        AssignmentOperator::Division => Some(HirBinOp::Div),
                        AssignmentOperator::Remainder => Some(HirBinOp::Rem),
                        AssignmentOperator::ShiftLeft => Some(HirBinOp::Shl),
                        AssignmentOperator::ShiftRight => Some(HirBinOp::Shr),
                        AssignmentOperator::BitwiseAnd => Some(HirBinOp::BitAnd),
                        AssignmentOperator::BitwiseOR => Some(HirBinOp::BitOr),
                        AssignmentOperator::BitwiseXOR => Some(HirBinOp::BitXor),
                        _ => None,
                    };
                    if let Some(op) = bin_op {
                        let expr = HirExpr::BinOp {
                            op,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr { var: var_id, expr });
                    } else {
                        return Err(EudkitTsError::Compile(format!(
                            "unsupported assignment operator {:?}",
                            assign.operator
                        )));
                    }
                }
            }
            return Ok(());
        }

        let rhs_expr = self.lower_expr(&assign.right)?;

        match &assign.left {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                let var_name = ident.name.as_str();
                let Some(&var_id) = self.vars.get(var_name) else {
                    return Err(EudkitTsError::Compile(format!(
                        "unknown variable {var_name}"
                    )));
                };

                match assign.operator {
                    AssignmentOperator::Assign => {
                        if let HirExpr::Const(c) = rhs_expr {
                            out.push(HirStmt::VarAssignConst {
                                var: var_id,
                                value: c,
                            });
                        } else {
                            out.push(HirStmt::VarAssignExpr {
                                var: var_id,
                                expr: rhs_expr,
                            });
                        }
                    }
                    AssignmentOperator::Addition => {
                        if let HirExpr::Const(c) = rhs_expr {
                            out.push(HirStmt::VarAddConst {
                                var: var_id,
                                value: c,
                            });
                        } else {
                            let add_expr = HirExpr::BinOp {
                                op: HirBinOp::Add,
                                lhs: Box::new(HirExpr::Var(var_id)),
                                rhs: Box::new(rhs_expr),
                            }
                            .fold_constants();
                            out.push(HirStmt::VarAssignExpr {
                                var: var_id,
                                expr: add_expr,
                            });
                        }
                    }
                    AssignmentOperator::Subtraction => {
                        if let HirExpr::Const(c) = rhs_expr {
                            out.push(HirStmt::VarSubConst {
                                var: var_id,
                                value: c,
                            });
                        } else {
                            let sub_expr = HirExpr::BinOp {
                                op: HirBinOp::Sub,
                                lhs: Box::new(HirExpr::Var(var_id)),
                                rhs: Box::new(rhs_expr),
                            }
                            .fold_constants();
                            out.push(HirStmt::VarAssignExpr {
                                var: var_id,
                                expr: sub_expr,
                            });
                        }
                    }
                    AssignmentOperator::BitwiseAnd => {
                        let expr = HirExpr::BinOp {
                            op: HirBinOp::BitAnd,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr { var: var_id, expr });
                    }
                    AssignmentOperator::BitwiseOR => {
                        let expr = HirExpr::BinOp {
                            op: HirBinOp::BitOr,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr { var: var_id, expr });
                    }
                    AssignmentOperator::BitwiseXOR => {
                        let expr = HirExpr::BinOp {
                            op: HirBinOp::BitXor,
                            lhs: Box::new(HirExpr::Var(var_id)),
                            rhs: Box::new(rhs_expr),
                        }
                        .fold_constants();
                        out.push(HirStmt::VarAssignExpr { var: var_id, expr });
                    }
                    _ => {
                        return Err(EudkitTsError::Compile(format!(
                            "unsupported assignment operator {:?}",
                            assign.operator
                        )));
                    }
                }
            }
            AssignmentTarget::StaticMemberExpression(mem) => {
                // Support CUnit property updates: e.g. `u.hitPoints = 1000` or `u.hitPoints += 10`
                let prop_name = mem.property.name.as_str();
                let obj_expr = if let Expression::Identifier(obj_id) = &mem.object {
                    if let Some(e) = self.struct_vars.get(obj_id.name.as_str()) {
                        e.clone()
                    } else {
                        self.lower_expr(&mem.object)?
                    }
                } else {
                    self.lower_expr(&mem.object)?
                };

                let (offset, _is_word) = match prop_name {
                    "hitPoints" | "hp" => (2, false), // EPD + 2 (0x008)
                    "shields" => (24, false),         // EPD + 24 (0x060)
                    "energy" => (40, true),           // EPD + 40 (0x0A2)
                    "unitType" => (25, true),         // EPD + 25 (0x064)
                    _ => {
                        return Err(EudkitTsError::Compile(format!(
                            "unsupported property assignment on struct: {prop_name}"
                        )));
                    }
                };

                let epd_target = HirExpr::BinOp {
                    op: HirBinOp::Add,
                    lhs: Box::new(obj_expr),
                    rhs: Box::new(HirExpr::Const(offset)),
                }
                .fold_constants();

                let modifier = match assign.operator {
                    AssignmentOperator::Assign => Modifier::SetTo,
                    AssignmentOperator::Addition => Modifier::Add,
                    AssignmentOperator::Subtraction => Modifier::Subtract,
                    _ => Modifier::SetTo,
                };

                out.push(HirStmt::Action(StockAction::SetDeathsDynamic {
                    player: epd_target,
                    modifier,
                    value: rhs_expr,
                    unit: 0,
                    mask: None,
                }));
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_update(&mut self, upd: &UpdateExpression, out: &mut Vec<HirStmt>) -> Result<()> {
        let var_id = match &upd.argument {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                let var_name = ident.name.as_str();
                let Some(&vid) = self.vars.get(var_name) else {
                    return Err(EudkitTsError::Compile(format!(
                        "unknown variable {var_name}"
                    )));
                };
                vid
            }
            SimpleAssignmentTarget::ComputedMemberExpression(comp) => {
                if let Expression::Identifier(ident) = &comp.object {
                    let idx = self.eval_const_expr(&comp.expression)?;
                    let var_name = format!("{}_{}", ident.name.as_str(), idx);
                    let Some(&vid) = self.vars.get(&var_name) else {
                        return Err(EudkitTsError::Compile(format!(
                            "unknown variable {var_name}"
                        )));
                    };
                    vid
                } else {
                    return Ok(());
                }
            }
            _ => return Ok(()),
        };

        match upd.operator {
            UpdateOperator::Increment => {
                out.push(HirStmt::VarAddConst {
                    var: var_id,
                    value: 1,
                });
            }
            UpdateOperator::Decrement => {
                out.push(HirStmt::VarSubConst {
                    var: var_id,
                    value: 1,
                });
            }
        }
        Ok(())
    }

    fn try_lower_reactive_primitive(
        &mut self,
        call: &CallExpression,
    ) -> Result<Option<Vec<HirStmt>>> {
        let fn_name = match &call.callee {
            Expression::Identifier(ident) => ident.name.as_str(),
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(obj) = &mem.object {
                    if obj.name.as_str() == "eud" || obj.name.as_str() == "eudkit" {
                        mem.property.name.as_str()
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };

        match fn_name {
            "once" => {
                let (cond_opt, cb_arg) = if call.arguments.len() == 1 {
                    (None, call.arguments.first())
                } else if call.arguments.len() >= 2 {
                    let first = call.arguments.first().and_then(|a| a.as_expression());
                    let is_null_or_undef = match first {
                        Some(Expression::NullLiteral(_)) => true,
                        Some(Expression::Identifier(id)) if id.name.as_str() == "undefined" => true,
                        _ => false,
                    };
                    if is_null_or_undef {
                        (None, call.arguments.get(1))
                    } else {
                        (first, call.arguments.get(1))
                    }
                } else {
                    return Ok(None);
                };

                let mut conds = Vec::new();
                if let Some(cond_expr) = cond_opt {
                    conds.extend(self.lower_condition(cond_expr)?);
                }
                let mut body = Vec::new();
                if let Some(cb) = cb_arg.and_then(|a| a.as_expression()) {
                    self.lower_callback(cb, &mut body)?;
                }
                Ok(Some(vec![HirStmt::Reactive(ReactiveRule::Once {
                    cond: conds,
                    body,
                })]))
            }
            "when" => {
                let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let Some(arg1) = call.arguments.get(1).and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let conds = self.lower_condition(arg0)?;
                let mut body = Vec::new();
                self.lower_callback(arg1, &mut body)?;
                Ok(Some(vec![HirStmt::Reactive(ReactiveRule::When {
                    cond: conds,
                    body,
                })]))
            }
            "until" => {
                let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let Some(arg1) = call.arguments.get(1).and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let conds = self.lower_condition(arg0)?;
                let mut body = Vec::new();
                self.lower_callback(arg1, &mut body)?;
                Ok(Some(vec![HirStmt::Reactive(ReactiveRule::Until {
                    cond: conds,
                    body,
                })]))
            }
            "after" => {
                let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let Some(arg1) = call.arguments.get(1).and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let loops_val = self.eval_const_expr(arg0)?;
                let mut body = Vec::new();
                self.lower_callback(arg1, &mut body)?;
                Ok(Some(vec![HirStmt::Reactive(ReactiveRule::After {
                    delay_ticks: loops_val as u32,
                    body,
                })]))
            }
            "every" => {
                let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let Some(arg1) = call.arguments.get(1).and_then(|a| a.as_expression()) else {
                    return Ok(None);
                };
                let loops_val = self.eval_const_expr(arg0)?;
                let mut body = Vec::new();
                self.lower_callback(arg1, &mut body)?;
                Ok(Some(vec![HirStmt::Reactive(ReactiveRule::Every {
                    period_ticks: loops_val as u32,
                    body,
                })]))
            }
            _ => Ok(None),
        }
    }

    fn lower_callback(&mut self, expr: &Expression, out: &mut Vec<HirStmt>) -> Result<()> {
        match expr {
            Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(b) => {
                    for stmt in &b.statements {
                        self.lower_stmt(stmt, out)?;
                    }
                }
                other => {
                    if let Some(e) = other.as_expression() {
                        self.lower_expr_stmt(e, out)?;
                    }
                }
            },
            Expression::FunctionExpression(f) => {
                if let Some(body) = &f.body {
                    for stmt in &body.statements {
                        self.lower_stmt(stmt, out)?;
                    }
                }
            }
            Expression::Identifier(id) => {
                if let Some(&fn_idx) = self.functions.get(id.name.as_str()) {
                    out.push(HirStmt::Call {
                        callee: eudkit_core::hir::FuncId(fn_idx as u32),
                        args: Vec::new(),
                        rets: Vec::new(),
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_condition(&mut self, expr: &Expression) -> Result<Vec<StockCondition>> {
        match expr {
            Expression::ParenthesizedExpression(paren) => self.lower_condition(&paren.expression),
            Expression::LogicalExpression(log) if log.operator == LogicalOperator::And => {
                let mut conds = self.lower_condition(&log.left)?;
                conds.extend(self.lower_condition(&log.right)?);
                Ok(conds)
            }
            Expression::ArrayExpression(arr) => {
                let mut conds = Vec::new();
                for elem in &arr.elements {
                    if let Some(e) = elem.as_expression() {
                        conds.extend(self.lower_condition(e)?);
                    }
                }
                Ok(conds)
            }
            Expression::BinaryExpression(bin) => {
                let lhs = self.lower_expr(&bin.left)?;
                let rhs = self.lower_expr(&bin.right)?;

                let op = match bin.operator {
                    BinaryOperator::GreaterEqualThan => CmpOp::Ge,
                    BinaryOperator::LessEqualThan => CmpOp::Le,
                    BinaryOperator::Equality | BinaryOperator::StrictEquality => CmpOp::Eq,
                    BinaryOperator::Inequality | BinaryOperator::StrictInequality => CmpOp::Ne,
                    BinaryOperator::GreaterThan => CmpOp::Gt,
                    BinaryOperator::LessThan => CmpOp::Lt,
                    _ => {
                        return Err(EudkitTsError::Compile(format!(
                            "unsupported binary comparison {:?}",
                            bin.operator
                        )));
                    }
                };

                Ok(vec![StockCondition::from_cmp(lhs, op, rhs)])
            }
            Expression::CallExpression(call) => {
                if let Some(cond) = self.try_lower_condition_call(call)? {
                    Ok(vec![cond])
                } else {
                    Err(EudkitTsError::Compile(
                        "unsupported condition call in expression".to_string(),
                    ))
                }
            }
            _ => Err(EudkitTsError::Compile(
                "unsupported expression in condition".to_string(),
            )),
        }
    }

    fn try_lower_condition_call(
        &mut self,
        call: &CallExpression,
    ) -> Result<Option<StockCondition>> {
        let (obj_name, fn_name) = match &call.callee {
            Expression::Identifier(ident) => (None, ident.name.as_str()),
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(obj_ident) = &mem.object {
                    (Some(obj_ident.name.as_str()), mem.property.name.as_str())
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };

        if let Some(obj) = obj_name {
            if obj != "eud" && obj != "session" && obj != "eudkit" {
                if let Some(&var_id) = self.vars.get(obj) {
                    let op = match fn_name {
                        "atLeast" => Some(CmpOp::Ge),
                        "atMost" => Some(CmpOp::Le),
                        "exactly" => Some(CmpOp::Eq),
                        _ => None,
                    };
                    if let Some(op) = op {
                        let arg = call
                            .arguments
                            .first()
                            .and_then(|a| a.as_expression())
                            .ok_or_else(|| {
                                EudkitTsError::Compile(format!("{fn_name} requires 1 argument"))
                            })?;
                        let rhs = self.lower_expr(arg)?;
                        return Ok(Some(StockCondition::from_cmp(
                            HirExpr::Var(var_id),
                            op,
                            rhs,
                        )));
                    }
                }
                return Ok(None);
            }
        }

        match fn_name {
            "Always" => Ok(Some(StockCondition::Always)),
            "Never" => Ok(Some(StockCondition::Never)),
            "IsUserCP" | "IsUserCp" => Ok(Some(StockCondition::IsUserCp)),
            "Deaths" | "DeathsX" => {
                let player_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("Deaths requires player".to_string()))?;
                let cmp_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("Deaths requires comparison".to_string())
                    })?;
                let amount_arg = call
                    .arguments
                    .get(2)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("Deaths requires amount".to_string()))?;

                let cmp = self.resolve_comparison(cmp_arg)?;
                let unit = if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression())
                {
                    self.resolve_unit_id(arg3)?
                } else {
                    0
                };
                let mask = if fn_name == "DeathsX" {
                    if let Some(arg4) = call.arguments.get(4).and_then(|a| a.as_expression()) {
                        Some(self.eval_const_expr(arg4)? as u32)
                    } else {
                        Some(0xFFFF_FFFF)
                    }
                } else {
                    None
                };

                let player_expr = self.lower_expr(player_arg)?;
                let amount_expr = self.lower_expr(amount_arg)?;

                if let (HirExpr::Const(p), HirExpr::Const(a)) = (&player_expr, &amount_expr) {
                    let pid = PlayerId::from_code(*p);
                    if let Some(m) = mask {
                        Ok(Some(StockCondition::DeathsX {
                            player: pid,
                            comparison: cmp,
                            amount: *a as u32,
                            unit,
                            mask: m,
                        }))
                    } else {
                        Ok(Some(StockCondition::Deaths {
                            player: pid,
                            comparison: cmp,
                            amount: *a as u32,
                            unit,
                        }))
                    }
                } else {
                    Ok(Some(StockCondition::DeathsDynamic {
                        player: player_expr,
                        comparison: cmp,
                        amount: amount_expr,
                        unit,
                        mask,
                    }))
                }
            }
            "Memory" | "MemoryX" => {
                let addr_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("Memory requires address".to_string()))?;
                let cmp_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("Memory requires comparison".to_string())
                    })?;
                let val_arg = call
                    .arguments
                    .get(2)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("Memory requires value".to_string()))?;

                let address = self.eval_const_expr(addr_arg)? as u32;
                let cmp = self.resolve_comparison(cmp_arg)?;
                let val_expr = self.lower_expr(val_arg)?;

                if fn_name == "MemoryX" {
                    let mask =
                        if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression()) {
                            self.eval_const_expr(arg3)? as u32
                        } else {
                            0xFFFF_FFFF
                        };
                    if let HirExpr::Const(c) = val_expr {
                        Ok(Some(StockCondition::MemoryX {
                            address,
                            comparison: cmp,
                            value: c as u32,
                            mask,
                        }))
                    } else {
                        Ok(Some(StockCondition::MemoryDynamic {
                            address,
                            comparison: cmp,
                            value: val_expr,
                            mask: Some(mask),
                        }))
                    }
                } else if let HirExpr::Const(c) = val_expr {
                    Ok(Some(StockCondition::Memory {
                        address,
                        comparison: cmp,
                        value: c as u32,
                    }))
                } else {
                    Ok(Some(StockCondition::MemoryDynamic {
                        address,
                        comparison: cmp,
                        value: val_expr,
                        mask: None,
                    }))
                }
            }
            "Accumulate" => {
                let p = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let cmp = self.resolve_comparison(call.arguments[1].as_expression().unwrap())?;
                let amt = self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u32;
                let res = if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression())
                {
                    self.resolve_resource(arg3)?
                } else {
                    ResourceType::Ore
                };
                Ok(Some(StockCondition::Accumulate {
                    player: PlayerId::from_code(p),
                    comparison: cmp,
                    amount: amt,
                    resource: res,
                }))
            }
            "Command" => {
                let p = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let cmp = self.resolve_comparison(call.arguments[1].as_expression().unwrap())?;
                let amt = self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u32;
                let unit = if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression())
                {
                    self.resolve_unit_id(arg3)?
                } else {
                    0
                };
                Ok(Some(StockCondition::Command {
                    player: PlayerId::from_code(p),
                    comparison: cmp,
                    amount: amt,
                    unit,
                }))
            }
            "Bring" => {
                let p = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let cmp = self.resolve_comparison(call.arguments[1].as_expression().unwrap())?;
                let amt = self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u32;
                let unit = self.resolve_unit_id(call.arguments[3].as_expression().unwrap())?;
                let loc = self.resolve_location_id(call.arguments[4].as_expression().unwrap())?;
                Ok(Some(StockCondition::Bring {
                    player: PlayerId::from_code(p),
                    comparison: cmp,
                    amount: amt,
                    unit,
                    location: loc,
                }))
            }
            "ElapsedTime" => {
                let cmp = self.resolve_comparison(call.arguments[0].as_expression().unwrap())?;
                let seconds =
                    self.eval_const_expr(call.arguments[1].as_expression().unwrap())? as u32;
                Ok(Some(StockCondition::ElapsedTime {
                    comparison: cmp,
                    seconds,
                }))
            }
            "Score" => {
                let p = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let score_type =
                    self.eval_const_expr(call.arguments[1].as_expression().unwrap())? as u8;
                let cmp = self.resolve_comparison(call.arguments[2].as_expression().unwrap())?;
                let amt = self.eval_const_expr(call.arguments[3].as_expression().unwrap())? as u32;
                Ok(Some(StockCondition::Score {
                    player: PlayerId::from_code(p),
                    score_type,
                    comparison: cmp,
                    amount: amt,
                }))
            }
            "Switch" => {
                let sw_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("Switch requires switch id".to_string())
                    })?;
                let state_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("Switch requires state".to_string()))?;
                let switch = self.eval_const_expr(sw_arg)? as u8;
                let state_val = self.eval_const_expr(state_arg)?;
                let state = u8::try_from(state_val)
                    .ok()
                    .and_then(SwitchState::from_code)
                    .ok_or_else(|| {
                        EudkitTsError::Compile(format!("invalid switch state {state_val}"))
                    })?;
                Ok(Some(StockCondition::Switch { switch, state }))
            }
            _ => Ok(None),
        }
    }

    fn try_lower_action(&mut self, call: &CallExpression) -> Result<Option<StockAction>> {
        let (obj_name, fn_name) = match &call.callee {
            Expression::Identifier(ident) => (None, ident.name.as_str()),
            Expression::StaticMemberExpression(mem) => {
                if let Expression::Identifier(obj_ident) = &mem.object {
                    (Some(obj_ident.name.as_str()), mem.property.name.as_str())
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };

        if let Some(obj) = obj_name {
            if let Some(template) = self.string_buffers.get(obj).cloned() {
                match fn_name {
                    "display" | "displayText" | "display_text" => {
                        let mut args = Vec::new();
                        for arg in &call.arguments {
                            if let Some(expr) = arg.as_expression() {
                                args.push(self.lower_expr(expr)?);
                            }
                        }
                        if args.is_empty() && !template.contains("%d") && !template.contains("%x") {
                            return Ok(Some(StockAction::DisplayText {
                                text: template,
                                always_display: true,
                            }));
                        } else {
                            let parts = parse_format_template(&template, &args)
                                .map_err(|e| EudkitTsError::Compile(e))?;
                            return Ok(Some(StockAction::FormatPrint {
                                target: PrintTarget::DisplayText,
                                parts,
                                always_display: true,
                            }));
                        }
                    }
                    "display_at" | "displayAt" | "print_at" | "PrintAt" | "DisplayTextAt" => {
                        let line = if let Some(arg0) =
                            call.arguments.first().and_then(|a| a.as_expression())
                        {
                            self.eval_const_expr(arg0)? as u8
                        } else {
                            0
                        };
                        let mut args = Vec::new();
                        for arg in call.arguments.iter().skip(1) {
                            if let Some(expr) = arg.as_expression() {
                                args.push(self.lower_expr(expr)?);
                            }
                        }
                        let parts = parse_format_template(&template, &args)
                            .map_err(|e| EudkitTsError::Compile(e))?;
                        return Ok(Some(StockAction::PrintAt { line, parts }));
                    }
                    _ => {}
                }
            }
            if obj != "eud" && obj != "session" && obj != "eudkit" {
                return Ok(None);
            }
        }

        match fn_name {
            "dwwrite_epd" => {
                let epd_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("dwwrite_epd requires epd".to_string())
                    })?;
                let val_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("dwwrite_epd requires value".to_string())
                    })?;
                let epd_expr = self.lower_expr(epd_arg)?;
                let val_expr = self.lower_expr(val_arg)?;
                if let (HirExpr::Const(p), HirExpr::Const(v)) = (&epd_expr, &val_expr) {
                    let pid = PlayerId::from_code(*p);
                    Ok(Some(StockAction::SetDeaths {
                        player: pid,
                        modifier: Modifier::SetTo,
                        value: HirValue::Const(*v),
                        unit: 0,
                    }))
                } else {
                    Ok(Some(StockAction::SetDeathsDynamic {
                        player: epd_expr,
                        modifier: Modifier::SetTo,
                        value: val_expr,
                        unit: 0,
                        mask: None,
                    }))
                }
            }
            "bwrite_epd" => {
                let epd_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("bwrite_epd requires epd".to_string()))?;
                let val_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("bwrite_epd requires value".to_string())
                    })?;
                let epd_expr = self.lower_expr(epd_arg)?;
                let val_expr = self.lower_expr(val_arg)?;
                Ok(Some(StockAction::SetDeathsDynamic {
                    player: epd_expr,
                    modifier: Modifier::SetTo,
                    value: val_expr,
                    unit: 0,
                    mask: Some(0xFF),
                }))
            }
            "wwrite_epd" => {
                let epd_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("wwrite_epd requires epd".to_string()))?;
                let val_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("wwrite_epd requires value".to_string())
                    })?;
                let epd_expr = self.lower_expr(epd_arg)?;
                let val_expr = self.lower_expr(val_arg)?;
                Ok(Some(StockAction::SetDeathsDynamic {
                    player: epd_expr,
                    modifier: Modifier::SetTo,
                    value: val_expr,
                    unit: 0,
                    mask: Some(0xFFFF),
                }))
            }
            "dwwrite" => {
                let addr_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("dwwrite requires address".to_string())
                    })?;
                let val_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| EudkitTsError::Compile("dwwrite requires value".to_string()))?;
                let addr = self.eval_const_expr(addr_arg)? as u32;
                let val_expr = self.lower_expr(val_arg)?;
                if let HirExpr::Const(v) = val_expr {
                    Ok(Some(StockAction::SetMemory {
                        address: addr,
                        modifier: Modifier::SetTo,
                        value: HirValue::Const(v),
                    }))
                } else {
                    Ok(Some(StockAction::SetMemoryDynamic {
                        address: addr,
                        modifier: Modifier::SetTo,
                        value: val_expr,
                        mask: None,
                    }))
                }
            }
            "SetDeaths" | "SetDeathsX" => {
                let p_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetDeaths requires player".to_string())
                    })?;
                let mod_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetDeaths requires modifier".to_string())
                    })?;
                let val_arg = call
                    .arguments
                    .get(2)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetDeaths requires value".to_string())
                    })?;

                let modifier = self.resolve_modifier(mod_arg)?;
                let unit = if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression())
                {
                    self.resolve_unit_id(arg3)?
                } else {
                    0
                };
                let mask = if fn_name == "SetDeathsX" {
                    if let Some(arg4) = call.arguments.get(4).and_then(|a| a.as_expression()) {
                        Some(self.eval_const_expr(arg4)? as u32)
                    } else {
                        Some(0xFFFF_FFFF)
                    }
                } else {
                    None
                };

                let p_expr = self.lower_expr(p_arg)?;
                let val_expr = self.lower_expr(val_arg)?;

                if let (HirExpr::Const(p), HirExpr::Const(v)) = (&p_expr, &val_expr) {
                    let pid = PlayerId::from_code(*p);
                    if let Some(m) = mask {
                        Ok(Some(StockAction::SetDeathsX {
                            player: pid,
                            modifier,
                            value: HirValue::Const(*v),
                            unit,
                            mask: m,
                        }))
                    } else {
                        Ok(Some(StockAction::SetDeaths {
                            player: pid,
                            modifier,
                            value: HirValue::Const(*v),
                            unit,
                        }))
                    }
                } else {
                    Ok(Some(StockAction::SetDeathsDynamic {
                        player: p_expr,
                        modifier,
                        value: val_expr,
                        unit,
                        mask,
                    }))
                }
            }
            "SetMemory" | "SetMemoryX" => {
                let addr_arg = call
                    .arguments
                    .first()
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetMemory requires address".to_string())
                    })?;
                let mod_arg = call
                    .arguments
                    .get(1)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetMemory requires modifier".to_string())
                    })?;
                let val_arg = call
                    .arguments
                    .get(2)
                    .and_then(|a| a.as_expression())
                    .ok_or_else(|| {
                        EudkitTsError::Compile("SetMemory requires value".to_string())
                    })?;

                let address = self.eval_const_expr(addr_arg)? as u32;
                let modifier = self.resolve_modifier(mod_arg)?;
                let val_expr = self.lower_expr(val_arg)?;

                if fn_name == "SetMemoryX" {
                    let mask =
                        if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression()) {
                            self.eval_const_expr(arg3)? as u32
                        } else {
                            0xFFFF_FFFF
                        };
                    if let HirExpr::Const(c) = val_expr {
                        Ok(Some(StockAction::SetMemoryX {
                            address,
                            modifier,
                            value: HirValue::Const(c),
                            mask,
                        }))
                    } else {
                        Ok(Some(StockAction::SetMemoryDynamic {
                            address,
                            modifier,
                            value: val_expr,
                            mask: Some(mask),
                        }))
                    }
                } else if let HirExpr::Const(c) = val_expr {
                    Ok(Some(StockAction::SetMemory {
                        address,
                        modifier,
                        value: HirValue::Const(c),
                    }))
                } else {
                    Ok(Some(StockAction::SetMemoryDynamic {
                        address,
                        modifier,
                        value: val_expr,
                        mask: None,
                    }))
                }
            }
            "SetResources" => {
                let p_arg = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let mod_arg = self.resolve_modifier(call.arguments[1].as_expression().unwrap())?;
                let amt_arg =
                    self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u32;
                let res_arg =
                    if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression()) {
                        self.resolve_resource(arg3)?
                    } else {
                        ResourceType::Ore
                    };
                Ok(Some(StockAction::SetResources {
                    player: PlayerId::from_code(p_arg),
                    modifier: mod_arg,
                    amount: amt_arg,
                    resource: res_arg,
                }))
            }
            "DisplayText" => {
                let text = if let Some(Expression::StringLiteral(slit)) =
                    call.arguments[0].as_expression()
                {
                    slit.value.as_str().unwrap_or("").to_string()
                } else {
                    "".to_string()
                };
                let always =
                    if let Some(arg1) = call.arguments.get(1).and_then(|a| a.as_expression()) {
                        if let Expression::BooleanLiteral(blit) = arg1 {
                            blit.value
                        } else {
                            true
                        }
                    } else {
                        true
                    };
                Ok(Some(StockAction::DisplayText {
                    text,
                    always_display: always,
                }))
            }
            "f_eprintln" | "displayText" => {
                let text = if let Some(Expression::StringLiteral(slit)) =
                    call.arguments.first().and_then(|a| a.as_expression())
                {
                    slit.value.as_str().unwrap_or("").to_string()
                } else {
                    "".to_string()
                };
                let mut args = Vec::new();
                for arg in call.arguments.iter().skip(1) {
                    if let Some(expr) = arg.as_expression() {
                        args.push(self.lower_expr(expr)?);
                    }
                }
                let parts =
                    parse_format_template(&text, &args).map_err(|e| EudkitTsError::Compile(e))?;
                Ok(Some(StockAction::FormatPrint {
                    target: PrintTarget::DisplayText,
                    parts,
                    always_display: true,
                }))
            }
            "print_at" | "PrintAt" | "display_at" | "displayAt" | "DisplayTextAt" => {
                let line =
                    if let Some(arg0) = call.arguments.first().and_then(|a| a.as_expression()) {
                        self.eval_const_expr(arg0)? as u8
                    } else {
                        0
                    };
                let text = if let Some(Expression::StringLiteral(slit)) =
                    call.arguments.get(1).and_then(|a| a.as_expression())
                {
                    slit.value.as_str().unwrap_or("").to_string()
                } else {
                    "".to_string()
                };
                let mut args = Vec::new();
                for arg in call.arguments.iter().skip(2) {
                    if let Some(expr) = arg.as_expression() {
                        args.push(self.lower_expr(expr)?);
                    }
                }
                let parts =
                    parse_format_template(&text, &args).map_err(|e| EudkitTsError::Compile(e))?;
                Ok(Some(StockAction::PrintAt { line, parts }))
            }
            "CreateUnit" => {
                let count = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let loc = self.resolve_location_id(call.arguments[2].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[3].as_expression().unwrap())? as u8;
                Ok(Some(StockAction::CreateUnit {
                    count,
                    unit,
                    location: loc,
                    player: PlayerId::from_code(i32::from(player)),
                }))
            }
            "KillUnitAt" => {
                let count = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let loc = self.resolve_location_id(call.arguments[2].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[3].as_expression().unwrap())? as u8;
                Ok(Some(StockAction::KillUnitAt {
                    count,
                    unit,
                    location: loc,
                    player: PlayerId::from_code(i32::from(player)),
                }))
            }
            "MinimapPing" => {
                let loc = self.resolve_location_id(call.arguments[0].as_expression().unwrap())?;
                Ok(Some(StockAction::MinimapPing { location: loc }))
            }
            "SetScore" => {
                let p = self.eval_const_expr(call.arguments[0].as_expression().unwrap())?;
                let modifier = self.resolve_modifier(call.arguments[1].as_expression().unwrap())?;
                let amount =
                    self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u32;
                let score_type =
                    if let Some(arg3) = call.arguments.get(3).and_then(|a| a.as_expression()) {
                        self.eval_const_expr(arg3)? as u8
                    } else {
                        0
                    };
                Ok(Some(StockAction::SetScore {
                    player: PlayerId::from_code(p),
                    modifier,
                    amount,
                    score_type,
                }))
            }
            "SetSwitch" => {
                let sw = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let action_code =
                    self.eval_const_expr(call.arguments[1].as_expression().unwrap())?;
                let action = u8::try_from(action_code)
                    .ok()
                    .and_then(SwitchAction::from_code)
                    .ok_or_else(|| {
                        EudkitTsError::Compile(format!("invalid switch action {action_code}"))
                    })?;
                Ok(Some(StockAction::SetSwitch { switch: sw, action }))
            }
            "MoveLocation" => {
                let loc = self.resolve_location_id(call.arguments[0].as_expression().unwrap())?;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u8;
                let dest_loc =
                    self.resolve_location_id(call.arguments[3].as_expression().unwrap())?;
                Ok(Some(StockAction::MoveLocation {
                    location: loc,
                    unit,
                    player: PlayerId::from_code(i32::from(player)),
                    dest_location: dest_loc,
                }))
            }
            "RemoveUnitAt" => {
                let count = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let loc = self.resolve_location_id(call.arguments[2].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[3].as_expression().unwrap())? as u8;
                Ok(Some(StockAction::RemoveUnitAt {
                    count,
                    unit,
                    location: loc,
                    player: PlayerId::from_code(i32::from(player)),
                }))
            }
            "KillUnit" => {
                let unit = self.resolve_unit_id(call.arguments[0].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[1].as_expression().unwrap())? as u8;
                Ok(Some(StockAction::KillUnit {
                    unit,
                    player: PlayerId::from_code(i32::from(player)),
                }))
            }
            "MoveUnit" => {
                let count = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u8;
                let start_loc =
                    self.resolve_location_id(call.arguments[3].as_expression().unwrap())?;
                let dest_loc =
                    self.resolve_location_id(call.arguments[4].as_expression().unwrap())?;
                Ok(Some(StockAction::MoveUnit {
                    count,
                    unit,
                    player: PlayerId::from_code(i32::from(player)),
                    start_location: start_loc,
                    dest_location: dest_loc,
                }))
            }
            "GiveUnits" => {
                let count = self.eval_const_expr(call.arguments[0].as_expression().unwrap())? as u8;
                let unit = self.resolve_unit_id(call.arguments[1].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[2].as_expression().unwrap())? as u8;
                let loc = self.resolve_location_id(call.arguments[3].as_expression().unwrap())?;
                let new_player =
                    self.eval_const_expr(call.arguments[4].as_expression().unwrap())? as u8;
                Ok(Some(StockAction::GiveUnits {
                    count,
                    unit,
                    player: PlayerId::from_code(i32::from(player)),
                    location: loc,
                    new_player: PlayerId::from_code(i32::from(new_player)),
                }))
            }
            "Order" => {
                let unit = self.resolve_unit_id(call.arguments[0].as_expression().unwrap())?;
                let player =
                    self.eval_const_expr(call.arguments[1].as_expression().unwrap())? as u8;
                let start_loc =
                    self.resolve_location_id(call.arguments[2].as_expression().unwrap())?;
                let order_type =
                    self.eval_const_expr(call.arguments[3].as_expression().unwrap())? as u8;
                let dest_loc =
                    self.resolve_location_id(call.arguments[4].as_expression().unwrap())?;
                Ok(Some(StockAction::Order {
                    unit,
                    player: PlayerId::from_code(i32::from(player)),
                    start_location: start_loc,
                    order_type,
                    dest_location: dest_loc,
                }))
            }
            "PlayWAV" => {
                let path = if let Some(Expression::StringLiteral(slit)) =
                    call.arguments.get(0).and_then(|a| a.as_expression())
                {
                    slit.value.as_str().unwrap_or("").to_string()
                } else {
                    String::new()
                };
                Ok(Some(StockAction::PlayWAV { path }))
            }
            "Comment" => {
                let text = if let Some(Expression::StringLiteral(slit)) =
                    call.arguments[0].as_expression()
                {
                    slit.value.as_str().unwrap_or("").to_string()
                } else {
                    "".to_string()
                };
                Ok(Some(StockAction::Comment { text }))
            }
            "PreserveTrigger" => Ok(Some(StockAction::PreserveTrigger)),
            "Victory" => Ok(Some(StockAction::Victory)),
            "Defeat" => Ok(Some(StockAction::Defeat)),
            _ => Ok(None),
        }
    }

    fn lower_expr(&mut self, expr: &Expression) -> Result<HirExpr> {
        match expr {
            Expression::NumericLiteral(num) => Ok(HirExpr::Const(num.value as i32)),
            Expression::Identifier(ident) => {
                let name = ident.name.as_str();
                if let Some(&lval) = self.loop_vars.get(name) {
                    return Ok(HirExpr::Const(lval));
                }
                if let Some(&lval) = self.local_constants.get(name) {
                    return Ok(HirExpr::Const(lval));
                }
                if let Some(&cval) = self.constants.get(name) {
                    return Ok(HirExpr::Const(cval));
                }
                if let Some(bval) = Self::builtin_const(name) {
                    return Ok(HirExpr::Const(bval));
                }
                if let Some(&var_id) = self.vars.get(name) {
                    return Ok(HirExpr::Var(var_id));
                }
                if let Some(sexpr) = self.struct_vars.get(name) {
                    return Ok(sexpr.clone());
                }
                Err(EudkitTsError::Compile(format!(
                    "unresolved identifier {name}"
                )))
            }
            Expression::UnaryExpression(unary) => {
                let inner = self.lower_expr(&unary.argument)?;
                let op = match unary.operator {
                    UnaryOperator::UnaryNegation => HirUnaryOp::Neg,
                    UnaryOperator::BitwiseNot => HirUnaryOp::BitNot,
                    _ => {
                        return Err(EudkitTsError::Compile(format!(
                            "unsupported unary operator {:?}",
                            unary.operator
                        )));
                    }
                };
                Ok(HirExpr::UnaryOp {
                    op,
                    operand: Box::new(inner),
                }
                .fold_constants())
            }
            Expression::BinaryExpression(bin) => {
                let lhs = self.lower_expr(&bin.left)?;
                let rhs = self.lower_expr(&bin.right)?;
                let op = match bin.operator {
                    BinaryOperator::Addition => HirBinOp::Add,
                    BinaryOperator::Subtraction => HirBinOp::Sub,
                    BinaryOperator::Multiplication => HirBinOp::Mul,
                    BinaryOperator::Division => HirBinOp::Div,
                    BinaryOperator::Remainder => HirBinOp::Rem,
                    BinaryOperator::BitwiseAnd => HirBinOp::BitAnd,
                    BinaryOperator::BitwiseOR => HirBinOp::BitOr,
                    BinaryOperator::BitwiseXOR => HirBinOp::BitXor,
                    BinaryOperator::ShiftLeft => HirBinOp::Shl,
                    BinaryOperator::ShiftRight => HirBinOp::Shr,
                    _ => {
                        return Err(EudkitTsError::Compile(format!(
                            "operator {:?} in arithmetic expression",
                            bin.operator
                        )));
                    }
                };
                Ok(HirExpr::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
                .fold_constants())
            }
            Expression::ParenthesizedExpression(paren) => self.lower_expr(&paren.expression),
            Expression::CallExpression(call) => {
                // Handle `EPD(address)`
                if let Expression::Identifier(ident) = &call.callee {
                    if ident.name.as_str() == "EPD" {
                        let arg = call
                            .arguments
                            .first()
                            .and_then(|a| a.as_expression())
                            .ok_or_else(|| {
                                EudkitTsError::Compile("EPD requires address".to_string())
                            })?;
                        let addr_expr = self.lower_expr(arg)?;
                        if let HirExpr::Const(c) = addr_expr {
                            return Ok(HirExpr::Const(epd(c as u32)));
                        }
                    }
                } else if let Expression::StaticMemberExpression(mem) = &call.callee {
                    let prop = mem.property.name.as_str();
                    if prop == "get" || prop == "valueOf" {
                        return self.lower_expr(&mem.object);
                    }
                }
                Err(EudkitTsError::Compile(format!(
                    "unsupported call in expression: {:?}",
                    call.callee
                )))
            }
            Expression::StaticMemberExpression(mem) => {
                // e.g. `u.epd`
                let obj = self.lower_expr(&mem.object)?;
                let prop = mem.property.name.as_str();
                if prop == "epd" {
                    Ok(obj)
                } else {
                    Err(EudkitTsError::Compile(format!(
                        "unsupported struct property {prop}"
                    )))
                }
            }
            Expression::ComputedMemberExpression(comp) => {
                if let Expression::Identifier(ident) = &comp.object {
                    let name = ident.name.as_str();
                    if let Some(arr) = self.array_constants.get(name) {
                        let idx = self.eval_const_expr(&comp.expression)?;
                        if (idx as usize) < arr.len() {
                            return Ok(HirExpr::Const(arr[idx as usize]));
                        }
                    }
                    if let Ok(idx) = self.eval_const_expr(&comp.expression) {
                        let key = format!("{name}_{idx}");
                        if let Some(&var_id) = self.vars.get(&key) {
                            return Ok(HirExpr::Var(var_id));
                        }
                    }
                }
                Err(EudkitTsError::Compile(format!(
                    "unsupported expression {:?}",
                    expr
                )))
            }
            _ => Err(EudkitTsError::Compile(format!(
                "unsupported expression {:?}",
                expr
            ))),
        }
    }

    pub fn builtin_const(name: &str) -> Option<i32> {
        frontend_constant(name).or_else(|| (name == "All").then_some(0))
    }

    fn eval_const_expr(&self, expr: &Expression) -> Result<i32> {
        match expr {
            Expression::NumericLiteral(num) => Ok(num.value as i32),
            Expression::StringLiteral(slit) => {
                let s = slit.value.as_str().unwrap_or("");
                // If unit or location or switch name used in place of int
                if let Some(uid) = symbols::unit_id(self.map_data, self.canonical_symbols, s) {
                    return Ok(uid as i32);
                }
                if let Some(lid) = symbols::location_id(self.map_data, self.canonical_symbols, s) {
                    return Ok(lid as i32);
                }
                if let Some(sid) = symbols::switch_id(self.map_data, self.canonical_symbols, s) {
                    return Ok(sid as i32);
                }
                Err(EudkitTsError::Compile(format!(
                    "unresolved string constant {:?}",
                    s
                )))
            }
            Expression::Identifier(ident) => {
                let name = ident.name.as_str();
                if let Some(&lval) = self.loop_vars.get(name) {
                    return Ok(lval);
                }
                if let Some(&lval) = self.local_constants.get(name) {
                    return Ok(lval);
                }
                if let Some(&cval) = self.constants.get(name) {
                    return Ok(cval);
                }
                if let Some(bval) = Self::builtin_const(name) {
                    return Ok(bval);
                }
                Err(EudkitTsError::Compile(format!(
                    "unresolved constant {name}"
                )))
            }
            Expression::UnaryExpression(unary) => {
                let val = self.eval_const_expr(&unary.argument)?;
                match unary.operator {
                    UnaryOperator::UnaryNegation => Ok(-val),
                    UnaryOperator::BitwiseNot => Ok(!val),
                    _ => Err(EudkitTsError::Compile(format!(
                        "unsupported const unary operator {:?}",
                        unary.operator
                    ))),
                }
            }
            Expression::BinaryExpression(bin) => {
                let l = self.eval_const_expr(&bin.left)?;
                let r = self.eval_const_expr(&bin.right)?;
                match bin.operator {
                    BinaryOperator::Addition => Ok(l.wrapping_add(r)),
                    BinaryOperator::Subtraction => Ok(l.wrapping_sub(r)),
                    BinaryOperator::Multiplication => Ok(l.wrapping_mul(r)),
                    BinaryOperator::Division => Ok(if r == 0 { 0 } else { l / r }),
                    BinaryOperator::Remainder => Ok(if r == 0 { 0 } else { l % r }),
                    BinaryOperator::BitwiseAnd => Ok(l & r),
                    BinaryOperator::BitwiseOR => Ok(l | r),
                    BinaryOperator::BitwiseXOR => Ok(l ^ r),
                    BinaryOperator::ShiftLeft => Ok(l << (r & 31)),
                    BinaryOperator::ShiftRight => Ok(l >> (r & 31)),
                    _ => Err(EudkitTsError::Compile(format!(
                        "unsupported const binary operator {:?}",
                        bin.operator
                    ))),
                }
            }
            Expression::CallExpression(call) => {
                if let Expression::Identifier(ident) = &call.callee {
                    if ident.name.as_str() == "EPD" {
                        let arg = call
                            .arguments
                            .first()
                            .and_then(|a| a.as_expression())
                            .ok_or_else(|| {
                                EudkitTsError::Compile("EPD requires address".to_string())
                            })?;
                        let addr = self.eval_const_expr(arg)?;
                        return Ok(epd(addr as u32));
                    }
                }
                Err(EudkitTsError::Compile(format!(
                    "unsupported const call {:?}",
                    call.callee
                )))
            }
            Expression::ParenthesizedExpression(paren) => self.eval_const_expr(&paren.expression),
            Expression::ComputedMemberExpression(comp) => {
                if let Expression::Identifier(ident) = &comp.object {
                    let name = ident.name.as_str();
                    if let Some(arr) = self.array_constants.get(name) {
                        let idx = self.eval_const_expr(&comp.expression)?;
                        if (idx as usize) < arr.len() {
                            return Ok(arr[idx as usize]);
                        }
                    }
                }
                Err(EudkitTsError::Compile(format!(
                    "unsupported const computed member {:?}",
                    comp
                )))
            }
            _ => Err(EudkitTsError::Compile(format!(
                "expected constant expression, got {:?}",
                expr
            ))),
        }
    }

    fn resolve_comparison(&self, expr: &Expression) -> Result<Comparison> {
        let val = self.eval_const_expr(expr)?;
        u8::try_from(val)
            .ok()
            .and_then(Comparison::from_code)
            .ok_or_else(|| EudkitTsError::Compile(format!("invalid comparison value {val}")))
    }

    fn resolve_modifier(&self, expr: &Expression) -> Result<Modifier> {
        let val = self.eval_const_expr(expr)?;
        u8::try_from(val)
            .ok()
            .and_then(Modifier::from_code)
            .ok_or_else(|| EudkitTsError::Compile(format!("invalid modifier value {val}")))
    }

    fn resolve_resource(&self, expr: &Expression) -> Result<ResourceType> {
        let val = self.eval_const_expr(expr)?;
        u8::try_from(val)
            .ok()
            .and_then(ResourceType::from_code)
            .ok_or_else(|| EudkitTsError::Compile(format!("invalid resource type {val}")))
    }

    fn resolve_unit_id(&self, expr: &Expression) -> Result<u16> {
        if let Expression::StringLiteral(slit) = expr {
            let s = slit.value.as_str().unwrap_or("");
            if let Some(id) = symbols::unit_id(self.map_data, self.canonical_symbols, s) {
                return Ok(id);
            }
            return Err(EudkitTsError::Compile(format!("unknown unit name {:?}", s)));
        }
        let val = self.eval_const_expr(expr)?;
        Ok(val as u16)
    }

    fn resolve_location_id(&self, expr: &Expression) -> Result<u32> {
        if let Expression::StringLiteral(slit) = expr {
            let s = slit.value.as_str().unwrap_or("");
            if let Some(id) = symbols::location_id(self.map_data, self.canonical_symbols, s) {
                return Ok(id);
            }
            return Err(EudkitTsError::Compile(format!(
                "unknown location name {:?}",
                s
            )));
        }
        let val = self.eval_const_expr(expr)?;
        Ok(val as u32)
    }
}

pub fn compile_ts_to_hir(
    source: &str,
    module_name: &str,
    map_data: &MapData,
    canonical_symbols: &CanonicalSymbols,
) -> Result<HirModule> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if !ret.diagnostics.is_empty() {
        return Err(EudkitTsError::Parse(format!(
            "failed to parse TypeScript {module_name}: {:?}",
            ret.diagnostics
        )));
    }

    let compiler = TsCompiler::new(module_name, map_data, canonical_symbols);
    compiler.compile(&ret.program)
}
