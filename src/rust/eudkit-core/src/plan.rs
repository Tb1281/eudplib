//! Canonical, versioned serialization of a frontend-produced [`HirModule`].
//!
//! A plan is a data artifact, not executable code. It lets the Backend compile
//! a map without starting Python or Node.js, while the frontends keep their
//! native authoring model. See `docs/adr/0006-frozen-hir-plans.md`.

use crate::error::{EudkitError, Result};
use crate::hir::{
    ByteBufId, FuncId, FuncPtrId, HirExpr, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, ReactiveRule, StockAction, StockCondition, StringBufferId, VarId,
};
use serde::{Deserialize, Serialize};

pub const HIR_PLAN_FORMAT: &str = "eudkit-hir-plan";
pub const HIR_PLAN_SCHEMA_VERSION: u32 = 1;

/// Envelope around a canonical HIR module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HirPlan {
    pub format: String,
    pub schema_version: u32,
    pub module: HirModule,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<PlanMap>,
}

/// Identity of the Map Context a plan was synthesized against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanMap {
    /// Stable fingerprint of the input map's `scenario.chk`.
    pub fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PlanMap {
    pub fn new(fingerprint: impl Into<String>) -> Self {
        Self {
            fingerprint: fingerprint.into(),
            name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

impl HirPlan {
    pub fn new(module: HirModule) -> Self {
        Self {
            format: HIR_PLAN_FORMAT.to_string(),
            schema_version: HIR_PLAN_SCHEMA_VERSION,
            module,
            map: None,
        }
    }

    pub fn with_map(mut self, map: PlanMap) -> Self {
        self.map = Some(map);
        self
    }

    /// Pretty JSON is the default artifact form: reviewable in pull requests,
    /// diffable, and stable because HIR contains only ordered sequences.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| EudkitError::Ir(format!("failed to serialize HIR plan: {e}")))
    }

    /// Compact JSON is useful for size benchmarks and cache keys.
    pub fn to_json_compact(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| EudkitError::Ir(format!("failed to serialize HIR plan: {e}")))
    }

    pub fn from_json(text: &str) -> Result<Self> {
        let plan: Self = serde_json::from_str(text)
            .map_err(|e| EudkitError::Ir(format!("invalid HIR plan JSON: {e}")))?;
        plan.validate()?;
        Ok(plan)
    }

    /// Validates the envelope and every cross-reference in the module before
    /// the Backend indexes into it. Plans can come from disk, so a malformed
    /// plan must be a diagnostic rather than a panic.
    pub fn validate(&self) -> Result<()> {
        if self.format != HIR_PLAN_FORMAT {
            return Err(invalid(format!(
                "unsupported format '{}' (expected '{}')",
                self.format, HIR_PLAN_FORMAT
            )));
        }
        if self.schema_version != HIR_PLAN_SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported schema version {} (this build supports {})",
                self.schema_version, HIR_PLAN_SCHEMA_VERSION
            )));
        }
        validate_module(&self.module)
    }
}

/// FNV-1a 64-bit. This is a map identity check, not a security primitive.
pub fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn invalid(message: impl Into<String>) -> EudkitError {
    EudkitError::Ir(format!("invalid HIR plan: {}", message.into()))
}

fn validate_module(module: &HirModule) -> Result<()> {
    for (index, function) in module.functions.iter().enumerate() {
        if function.id.0 as usize != index {
            return Err(invalid(format!(
                "function @f{} is stored at index {index}",
                function.id.0
            )));
        }
        validate_function(module, function)?;
    }

    if let Some(main_loop) = module.main_loop {
        check_func(module, main_loop, "main_loop")?;
    }

    for (index, var) in module.vars.iter().enumerate() {
        if var.id.0 as usize != index {
            return Err(invalid(format!(
                "variable @v{} is stored at index {index}",
                var.id.0
            )));
        }
    }

    for (index, byte_buf) in module.byte_bufs.iter().enumerate() {
        if byte_buf.id.0 as usize != index {
            return Err(invalid(format!(
                "byte buffer @buf{} is stored at index {index}",
                byte_buf.id.0
            )));
        }
    }

    for (index, buffer) in module.string_buffers.iter().enumerate() {
        if buffer.id.0 as usize != index {
            return Err(invalid(format!(
                "string buffer @strbuf{} is stored at index {index}",
                buffer.id.0
            )));
        }
    }

    for (index, func_ptr) in module.func_ptrs.iter().enumerate() {
        if func_ptr.id.0 as usize != index {
            return Err(invalid(format!(
                "function pointer @fp{} is stored at index {index}",
                func_ptr.id.0
            )));
        }
        if let Some(target) = func_ptr.initial_target {
            check_func(module, target, "function pointer initial_target")?;
        }
    }

    Ok(())
}

fn validate_function(module: &HirModule, function: &HirFunction) -> Result<()> {
    for param in &function.params {
        check_var(module, *param, "function parameter")?;
    }

    for (index, block) in function.blocks.iter().enumerate() {
        if block.id.0 as usize != index {
            return Err(invalid(format!(
                "block @b{} in function '{}' is stored at index {index}",
                block.id.0, function.name
            )));
        }
        for stmt in &block.stmts {
            validate_stmt(module, stmt)?;
        }
        validate_terminator(module, function, &block.terminator)?;
    }

    Ok(())
}

fn validate_terminator(
    module: &HirModule,
    function: &HirFunction,
    terminator: &HirTerminator,
) -> Result<()> {
    match terminator {
        HirTerminator::Jump(target) => check_block(function, *target),
        HirTerminator::Branch {
            conditions,
            then_block,
            else_block,
        } => {
            for condition in conditions {
                validate_condition(module, condition)?;
            }
            check_block(function, *then_block)?;
            check_block(function, *else_block)
        }
        HirTerminator::Return => Ok(()),
    }
}

fn validate_stmt(module: &HirModule, stmt: &HirStmt) -> Result<()> {
    match stmt {
        HirStmt::Trigger {
            conditions,
            actions,
            ..
        } => {
            for condition in conditions {
                validate_condition(module, condition)?;
            }
            for action in actions {
                validate_action(module, action)?;
            }
            Ok(())
        }
        HirStmt::Action(action) => validate_action(module, action),
        HirStmt::VarAssignConst { var, .. }
        | HirStmt::VarAddConst { var, .. }
        | HirStmt::VarSubConst { var, .. }
        | HirStmt::VarSetMaskConst { var, .. } => check_var(module, *var, "statement"),
        HirStmt::VarAssignExpr { var, expr } => {
            check_var(module, *var, "assignment target")?;
            validate_expr(module, expr)
        }
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            for condition in conditions {
                validate_condition(module, condition)?;
            }
            for nested in then_body.iter().chain(else_body) {
                validate_stmt(module, nested)?;
            }
            Ok(())
        }
        HirStmt::While { conditions, body } => {
            for condition in conditions {
                validate_condition(module, condition)?;
            }
            for nested in body {
                validate_stmt(module, nested)?;
            }
            Ok(())
        }
        HirStmt::Break | HirStmt::Continue => Ok(()),
        HirStmt::Call { callee, args, rets } => {
            check_func(module, *callee, "call target")?;
            for arg in args {
                validate_expr(module, arg)?;
            }
            for ret in rets {
                check_var(module, *ret, "call return")?;
            }
            Ok(())
        }
        HirStmt::ReturnValues(values) => {
            for value in values {
                validate_expr(module, value)?;
            }
            Ok(())
        }
        HirStmt::FuncPtrAssign { ptr, target } => {
            check_func_ptr(module, *ptr)?;
            check_func(module, *target, "function pointer target")
        }
        HirStmt::CallIndirect { ptr, args, rets } => {
            check_func_ptr(module, *ptr)?;
            for arg in args {
                validate_expr(module, arg)?;
            }
            for ret in rets {
                check_var(module, *ret, "indirect call return")?;
            }
            Ok(())
        }
        HirStmt::MemRead { dst, epd, .. } => {
            check_var(module, *dst, "memory read destination")?;
            validate_expr(module, epd)
        }
        HirStmt::Reactive(rule) => validate_reactive(module, rule),
    }
}

fn validate_reactive(module: &HirModule, rule: &ReactiveRule) -> Result<()> {
    let (conditions, body) = match rule {
        ReactiveRule::Once { cond, body }
        | ReactiveRule::When { cond, body }
        | ReactiveRule::Until { cond, body } => (Some(cond), body),
        ReactiveRule::After { body, .. } | ReactiveRule::Every { body, .. } => (None, body),
    };

    if let Some(conditions) = conditions {
        for condition in conditions {
            validate_condition(module, condition)?;
        }
    }
    for stmt in body {
        validate_stmt(module, stmt)?;
    }
    Ok(())
}

fn validate_condition(module: &HirModule, condition: &StockCondition) -> Result<()> {
    match condition {
        StockCondition::Always | StockCondition::Never | StockCondition::IsUserCp => Ok(()),
        StockCondition::DeathsDynamic { player, amount, .. } => {
            validate_expr(module, player)?;
            validate_expr(module, amount)
        }
        StockCondition::MemoryDynamic { value, .. } => validate_expr(module, value),
        StockCondition::VarCmp { var, .. } => check_var(module, *var, "condition"),
        StockCondition::ExprCmp { lhs, rhs, .. } => {
            validate_expr(module, lhs)?;
            validate_expr(module, rhs)
        }
        StockCondition::Deaths { .. }
        | StockCondition::DeathsX { .. }
        | StockCondition::Memory { .. }
        | StockCondition::MemoryX { .. }
        | StockCondition::Accumulate { .. }
        | StockCondition::Command { .. }
        | StockCondition::Bring { .. }
        | StockCondition::Switch { .. }
        | StockCondition::ElapsedTime { .. }
        | StockCondition::Score { .. } => Ok(()),
    }
}

fn validate_action(module: &HirModule, action: &StockAction) -> Result<()> {
    match action {
        StockAction::SetDeaths { value, .. } | StockAction::SetDeathsX { value, .. } => {
            validate_value(module, value)
        }
        StockAction::SetDeathsDynamic { player, value, .. } => {
            validate_expr(module, player)?;
            validate_expr(module, value)
        }
        StockAction::SetMemory { value, .. } | StockAction::SetMemoryX { value, .. } => {
            validate_value(module, value)
        }
        StockAction::SetMemoryDynamic { value, .. } => validate_expr(module, value),
        StockAction::SetVarConst { var, .. } => check_var(module, *var, "action"),
        StockAction::SetField { value, .. } => validate_value(module, value),
        StockAction::SetFieldDynamic { value, .. } => validate_expr(module, value),
        StockAction::FormatPrint { parts, .. } | StockAction::PrintAt { parts, .. } => {
            for part in parts {
                match part {
                    crate::hir::FormatPart::Text(_) => {}
                    crate::hir::FormatPart::Decimal(expr) | crate::hir::FormatPart::Hex(expr) => {
                        validate_expr(module, expr)?
                    }
                }
            }
            Ok(())
        }
        StockAction::DisplayStringBuffer { buffer, .. } => {
            check_string_buffer(module, *buffer, "action")
        }
        StockAction::FormatStringBuffer { buffer, parts, .. } => {
            check_string_buffer(module, *buffer, "action")?;
            for part in parts {
                match part {
                    crate::hir::FormatPart::Text(_) => {}
                    crate::hir::FormatPart::Decimal(expr) | crate::hir::FormatPart::Hex(expr) => {
                        validate_expr(module, expr)?
                    }
                }
            }
            Ok(())
        }
        StockAction::SetResources { .. }
        | StockAction::DisplayText { .. }
        | StockAction::DisplayTextId { .. }
        | StockAction::CreateUnit { .. }
        | StockAction::KillUnitAt { .. }
        | StockAction::MinimapPing { .. }
        | StockAction::SetScore { .. }
        | StockAction::SetSwitch { .. }
        | StockAction::MoveLocation { .. }
        | StockAction::RemoveUnitAt { .. }
        | StockAction::KillUnit { .. }
        | StockAction::MoveUnit { .. }
        | StockAction::GiveUnits { .. }
        | StockAction::Order { .. }
        | StockAction::PlayWAV { .. }
        | StockAction::Comment { .. }
        | StockAction::PreserveTrigger
        | StockAction::Victory
        | StockAction::Defeat => Ok(()),
    }
}

fn validate_value(module: &HirModule, value: &HirValue) -> Result<()> {
    match value {
        HirValue::Const(_) => Ok(()),
        HirValue::ByteBufAddress(id, _) | HirValue::ByteBufEpd(id, _) => {
            check_byte_buf(module, *id)
        }
        HirValue::VarAddress(id) | HirValue::VarEpd(id) => check_var(module, *id, "value"),
        // FieldRef object IDs are LIR object indices assigned after lowering.
        HirValue::FieldAddress(_) | HirValue::FieldEpd(_) => Ok(()),
    }
}

fn validate_expr(module: &HirModule, expr: &HirExpr) -> Result<()> {
    match expr {
        HirExpr::Const(_) => Ok(()),
        HirExpr::Var(var) => check_var(module, *var, "expression"),
        HirExpr::UnaryOp { operand, .. } => validate_expr(module, operand),
        HirExpr::BinOp { lhs, rhs, .. } => {
            validate_expr(module, lhs)?;
            validate_expr(module, rhs)
        }
    }
}

fn check_var(module: &HirModule, id: VarId, context: &str) -> Result<()> {
    if (id.0 as usize) < module.vars.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "{context} references @v{} but the module has {} variables",
            id.0,
            module.vars.len()
        )))
    }
}

fn check_func(module: &HirModule, id: FuncId, context: &str) -> Result<()> {
    if (id.0 as usize) < module.functions.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "{context} references @f{} but the module has {} functions",
            id.0,
            module.functions.len()
        )))
    }
}

fn check_byte_buf(module: &HirModule, id: ByteBufId) -> Result<()> {
    if (id.0 as usize) < module.byte_bufs.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "byte buffer reference @buf{} but the module has {} buffers",
            id.0,
            module.byte_bufs.len()
        )))
    }
}

fn check_string_buffer(module: &HirModule, id: StringBufferId, context: &str) -> Result<()> {
    if (id.0 as usize) < module.string_buffers.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "{context} references @strbuf{} but the module has {} string buffers",
            id.0,
            module.string_buffers.len()
        )))
    }
}

fn check_func_ptr(module: &HirModule, id: FuncPtrId) -> Result<()> {
    if (id.0 as usize) < module.func_ptrs.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "function pointer reference @fp{} but the module has {} pointers",
            id.0,
            module.func_ptrs.len()
        )))
    }
}

fn check_block(function: &HirFunction, id: crate::hir::BlockId) -> Result<()> {
    if (id.0 as usize) < function.blocks.len() {
        Ok(())
    } else {
        Err(invalid(format!(
            "block reference @b{} in function '{}' but the function has {} blocks",
            id.0,
            function.name,
            function.blocks.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{Comparison, HirBasicBlock};

    fn sample_module() -> HirModule {
        let mut module = HirModule::new("plan_test");
        let var = module.add_var("counter", 0);
        let mut function = HirFunction::new_main_loop("main");
        let block = function.add_block(HirBasicBlock::new("entry"));
        function
            .block_mut(block)
            .push_stmt(HirStmt::VarAddConst { var, value: 1 });
        function.block_mut(block).push_stmt(HirStmt::If {
            conditions: vec![StockCondition::VarCmp {
                var,
                comparison: Comparison::AtLeast,
                value: 10,
                mask: None,
            }],
            then_body: vec![HirStmt::Action(StockAction::Victory)],
            else_body: Vec::new(),
        });
        module.add_function(function);
        module
    }

    #[test]
    fn plan_round_trips_and_serializes_deterministically() {
        let plan = HirPlan::new(sample_module()).with_map(PlanMap::new("0123456789abcdef"));
        let first = plan.to_json().unwrap();
        let second = plan.to_json().unwrap();
        assert_eq!(first, second);

        let parsed = HirPlan::from_json(&first).unwrap();
        assert_eq!(parsed, plan);
    }

    #[test]
    fn compact_json_is_smaller_than_pretty_json() {
        let plan = HirPlan::new(sample_module());
        let pretty = plan.to_json().unwrap();
        let compact = plan.to_json_compact().unwrap();
        assert!(compact.len() < pretty.len());
    }

    #[test]
    fn invalid_schema_version_is_rejected() {
        let mut plan = HirPlan::new(sample_module());
        plan.schema_version = HIR_PLAN_SCHEMA_VERSION + 1;
        let json = serde_json::to_string(&plan).unwrap();
        let error = HirPlan::from_json(&json).unwrap_err().to_string();
        assert!(error.contains("unsupported schema version"), "{error}");
    }

    #[test]
    fn out_of_range_variable_is_rejected() {
        let mut plan = HirPlan::new(sample_module());
        plan.module.functions[0].blocks[0]
            .stmts
            .push(HirStmt::VarAddConst {
                var: VarId(99),
                value: 1,
            });
        let error = plan.validate().unwrap_err().to_string();
        assert!(error.contains("@v99"), "{error}");
    }

    #[test]
    fn fingerprint_is_stable_and_content_sensitive() {
        assert_eq!(fingerprint_bytes(b"abc"), fingerprint_bytes(b"abc"));
        assert_ne!(fingerprint_bytes(b"abc"), fingerprint_bytes(b"xyz"));
        assert_eq!(fingerprint_bytes(b"abc").len(), 16);
    }
}
