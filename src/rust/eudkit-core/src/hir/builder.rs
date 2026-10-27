use crate::error::{EudkitError, Result};
use crate::hir::{
    ByteBufId, FuncId, FuncPtrId, HirBasicBlock, HirByteBuf, HirExpr, HirFunction,
    HirModule, HirStmt, HirTerminator, InlineHint, Modifier, StockAction, StockCondition, VarId,
};

#[derive(Clone, Debug)]
pub enum OpenControlFrame {
    Function {
        name: String,
        is_main_loop: bool,
        params: Vec<VarId>,
        return_count: usize,
        inline_hint: InlineHint,
        body: Vec<HirStmt>,
    },
    IfThen {
        conditions: Vec<StockCondition>,
        then_body: Vec<HirStmt>,
    },
    Elif {
        conditions: Vec<StockCondition>,
        then_body: Vec<HirStmt>,
    },
    Else {
        else_body: Vec<HirStmt>,
    },
    While {
        conditions: Vec<StockCondition>,
        body: Vec<HirStmt>,
    },
}

#[derive(Clone, Debug)]
pub struct HirBuilder {
    hir: HirModule,
    root_stmts: Vec<HirStmt>,
    frames: Vec<OpenControlFrame>,
    last_was_else: bool,
}

impl HirBuilder {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            hir: HirModule::new(module_name),
            root_stmts: Vec::new(),
            frames: Vec::new(),
            last_was_else: false,
        }
    }

    pub fn from_module(hir: HirModule) -> Self {
        Self {
            hir,
            root_stmts: Vec::new(),
            frames: Vec::new(),
            last_was_else: false,
        }
    }

    pub fn module(&self) -> &HirModule {
        &self.hir
    }

    pub fn module_mut(&mut self) -> &mut HirModule {
        &mut self.hir
    }

    pub fn open_frames_count(&self) -> usize {
        self.frames.len()
    }

    pub fn is_in_frame(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn active_stmts_mut(&mut self) -> &mut Vec<HirStmt> {
        if let Some(top) = self.frames.last_mut() {
            match top {
                OpenControlFrame::Function { body, .. } => body,
                OpenControlFrame::IfThen { then_body, .. } => then_body,
                OpenControlFrame::Elif { then_body, .. } => then_body,
                OpenControlFrame::Else { else_body } => else_body,
                OpenControlFrame::While { body, .. } => body,
            }
        } else {
            &mut self.root_stmts
        }
    }

    pub fn add_var(&mut self, name: impl Into<String>, init_val: i32) -> VarId {
        self.hir.add_var(name, init_val)
    }

    pub fn add_byte_buf(&mut self, buf: HirByteBuf) -> ByteBufId {
        self.hir.add_byte_buf(buf)
    }

    pub fn add_func_ptr(
        &mut self,
        name: impl Into<String>,
        signature: (usize, usize),
        initial_target: Option<FuncId>,
    ) -> FuncPtrId {
        self.hir.add_func_ptr(name, signature, initial_target)
    }

    pub fn begin_function(
        &mut self,
        name: impl Into<String>,
        is_main_loop: bool,
        params: Vec<VarId>,
        return_count: usize,
        inline: Option<bool>,
    ) {
        let inline_hint = match inline {
            Some(true) => InlineHint::Always,
            Some(false) => InlineHint::Never,
            None => InlineHint::Auto,
        };
        self.last_was_else = false;
        self.frames.push(OpenControlFrame::Function {
            name: name.into(),
            is_main_loop,
            params,
            return_count,
            inline_hint,
            body: Vec::new(),
        });
    }

    pub fn end_function(&mut self) -> Result<FuncId> {
        match self.frames.pop() {
            Some(OpenControlFrame::Function {
                name,
                is_main_loop,
                params,
                return_count,
                inline_hint,
                body,
            }) => {
                let mut func = HirFunction::new(name, is_main_loop)
                    .with_params(params)
                    .with_return_count(return_count)
                    .with_inline_hint(inline_hint);
                let mut entry = HirBasicBlock::new("entry");
                entry.stmts = body;
                entry.set_terminator(HirTerminator::Return);
                func.add_block(entry);
                let fn_id = self.hir.add_function(func);
                self.last_was_else = false;
                Ok(fn_id)
            }
            _ => Err(EudkitError::Ir(
                "end_function without matching begin_function".into(),
            )),
        }
    }

    pub fn begin_if(&mut self, conditions: Vec<StockCondition>) {
        self.last_was_else = false;
        self.frames.push(OpenControlFrame::IfThen {
            conditions,
            then_body: Vec::new(),
        });
    }

    pub fn end_if(&mut self) -> Result<()> {
        match self.frames.pop() {
            Some(OpenControlFrame::IfThen {
                conditions,
                then_body,
            }) => {
                self.active_stmts_mut().push(HirStmt::If {
                    conditions,
                    then_body,
                    else_body: Vec::new(),
                });
                self.last_was_else = false;
                Ok(())
            }
            _ => Err(EudkitError::Ir("end_if without matching begin_if".into())),
        }
    }

    pub fn begin_elif(&mut self, conditions: Vec<StockCondition>) -> Result<()> {
        if self.last_was_else {
            return Err(EudkitError::Ir(
                "if block already has an else branch".into(),
            ));
        }
        can_attach_else(self.active_stmts_mut())?;
        self.frames.push(OpenControlFrame::Elif {
            conditions,
            then_body: Vec::new(),
        });
        Ok(())
    }

    pub fn end_elif(&mut self) -> Result<()> {
        match self.frames.pop() {
            Some(OpenControlFrame::Elif {
                conditions,
                then_body,
            }) => {
                let new_if = HirStmt::If {
                    conditions,
                    then_body,
                    else_body: Vec::new(),
                };
                attach_to_last_if_else(self.active_stmts_mut(), vec![new_if])?;
                self.last_was_else = false;
                Ok(())
            }
            _ => Err(EudkitError::Ir("end_elif without matching begin_elif".into())),
        }
    }

    pub fn begin_else(&mut self) -> Result<()> {
        if self.last_was_else {
            return Err(EudkitError::Ir(
                "if block already has an else branch".into(),
            ));
        }
        can_attach_else(self.active_stmts_mut())?;
        self.frames.push(OpenControlFrame::Else {
            else_body: Vec::new(),
        });
        Ok(())
    }

    pub fn end_else(&mut self) -> Result<()> {
        match self.frames.pop() {
            Some(OpenControlFrame::Else { else_body }) => {
                attach_to_last_if_else(self.active_stmts_mut(), else_body)?;
                self.last_was_else = true;
                Ok(())
            }
            _ => Err(EudkitError::Ir("end_else without matching begin_else".into())),
        }
    }

    pub fn begin_while(&mut self, conditions: Vec<StockCondition>) {
        self.last_was_else = false;
        self.frames.push(OpenControlFrame::While {
            conditions,
            body: Vec::new(),
        });
    }

    pub fn end_while(&mut self) -> Result<()> {
        match self.frames.pop() {
            Some(OpenControlFrame::While { conditions, body }) => {
                self.active_stmts_mut()
                    .push(HirStmt::While { conditions, body });
                self.last_was_else = false;
                Ok(())
            }
            _ => Err(EudkitError::Ir(
                "end_while without matching begin_while".into(),
            )),
        }
    }

    pub fn push_stmt(&mut self, stmt: HirStmt) {
        self.last_was_else = false;
        self.active_stmts_mut().push(stmt);
    }

    pub fn push_break(&mut self) {
        self.push_stmt(HirStmt::Break);
    }

    pub fn push_continue(&mut self) {
        self.push_stmt(HirStmt::Continue);
    }

    pub fn push_var_assign_const(&mut self, var: VarId, value: i32) {
        self.push_stmt(HirStmt::VarAssignConst { var, value });
    }

    pub fn push_var_assign_expr(&mut self, var: VarId, expr: HirExpr) {
        self.push_stmt(HirStmt::VarAssignExpr { var, expr });
    }

    pub fn push_var_add_const(&mut self, var: VarId, value: i32) {
        self.push_stmt(HirStmt::VarAddConst { var, value });
    }

    pub fn push_var_sub_const(&mut self, var: VarId, value: i32) {
        self.push_stmt(HirStmt::VarSubConst { var, value });
    }

    pub fn push_var_set_mask(&mut self, var: VarId, modifier: Modifier, value: i32, mask: u32) {
        self.push_stmt(HirStmt::VarSetMaskConst {
            var,
            modifier,
            value,
            mask,
        });
    }

    pub fn push_action(&mut self, action: StockAction) {
        self.push_stmt(HirStmt::Action(action));
    }

    pub fn push_trigger(
        &mut self,
        conditions: Vec<StockCondition>,
        actions: Vec<StockAction>,
        label: Option<String>,
        preserved: bool,
    ) {
        self.push_stmt(HirStmt::Trigger {
            label,
            conditions,
            actions,
            preserved,
        });
    }

    pub fn push_call(&mut self, callee: FuncId, args: Vec<HirExpr>, rets: Vec<VarId>) {
        self.push_stmt(HirStmt::Call { callee, args, rets });
    }

    pub fn push_return_values(&mut self, values: Vec<HirExpr>) {
        self.push_stmt(HirStmt::ReturnValues(values));
    }

    pub fn push_mem_read(
        &mut self,
        dst: VarId,
        epd: HirExpr,
        mask: u32,
        shift: i32,
        initial_val: i32,
    ) {
        self.push_stmt(HirStmt::MemRead {
            dst,
            epd,
            mask,
            shift,
            initial_val,
        });
    }

    pub fn push_func_ptr_assign(&mut self, ptr: FuncPtrId, target: FuncId) {
        self.push_stmt(HirStmt::FuncPtrAssign { ptr, target });
    }

    pub fn push_call_indirect(&mut self, ptr: FuncPtrId, args: Vec<HirExpr>, rets: Vec<VarId>) {
        self.push_stmt(HirStmt::CallIndirect { ptr, args, rets });
    }

    pub fn build(&self) -> Result<HirModule> {
        if !self.frames.is_empty() {
            return Err(EudkitError::Ir(
                "cannot compile or dump IR while a function or control-flow block is still open"
                    .into(),
            ));
        }
        let mut module = self.hir.clone();
        Self::finalize_module(&mut module, &self.root_stmts);
        Ok(module)
    }

    pub fn finish(mut self) -> Result<HirModule> {
        if !self.frames.is_empty() {
            return Err(EudkitError::Ir(
                "cannot finish HIR while a function or control-flow block is still open".into(),
            ));
        }
        Self::finalize_module(&mut self.hir, &self.root_stmts);
        Ok(self.hir)
    }

    fn finalize_module(module: &mut HirModule, root_stmts: &[HirStmt]) {
        if !root_stmts.is_empty() {
            if let Some(main_id) = module.main_loop {
                module.functions[main_id.0 as usize].blocks[0]
                    .stmts
                    .extend(root_stmts.iter().cloned());
            } else {
                let mut main_fn = HirFunction::new_main_loop("main");
                let mut entry = HirBasicBlock::new("entry");
                entry.stmts = root_stmts.to_vec();
                entry.set_terminator(HirTerminator::Return);
                main_fn.add_block(entry);
                module.add_function(main_fn);
            }
        } else if module.functions.is_empty() {
            let mut main_fn = HirFunction::new_main_loop("main");
            let entry = HirBasicBlock::new("entry");
            main_fn.add_block(entry);
            module.add_function(main_fn);
        }
    }
}

fn can_attach_else(stmts: &[HirStmt]) -> Result<()> {
    let Some(mut cur_stmt) = stmts.last() else {
        return Err(EudkitError::Ir(
            "elif / else must immediately follow an if block".into(),
        ));
    };
    loop {
        match cur_stmt {
            HirStmt::If { else_body, .. } => {
                if else_body.is_empty() {
                    return Ok(());
                } else if else_body.len() == 1 && matches!(else_body[0], HirStmt::If { .. }) {
                    cur_stmt = &else_body[0];
                } else {
                    return Err(EudkitError::Ir(
                        "if block already has an else branch".into(),
                    ));
                }
            }
            _ => {
                return Err(EudkitError::Ir(
                    "elif / else must immediately follow an if block".into(),
                ));
            }
        }
    }
}

fn attach_to_last_if_else(stmts: &mut [HirStmt], new_else: Vec<HirStmt>) -> Result<()> {
    let Some(mut cur_stmt) = stmts.last_mut() else {
        return Err(EudkitError::Ir(
            "elif / else must immediately follow an if block".into(),
        ));
    };
    loop {
        match cur_stmt {
            HirStmt::If { else_body, .. } => {
                if else_body.is_empty() {
                    *else_body = new_else;
                    return Ok(());
                } else if else_body.len() == 1 && matches!(else_body[0], HirStmt::If { .. }) {
                    cur_stmt = &mut else_body[0];
                } else {
                    return Err(EudkitError::Ir(
                        "if block already has an else branch".into(),
                    ));
                }
            }
            _ => {
                return Err(EudkitError::Ir(
                    "elif / else must immediately follow an if block".into(),
                ));
            }
        }
    }
}
