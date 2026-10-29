//! Debug presentation for HIR modules.
//!
//! `HirModule::dump_hir` is the public entry point; the helpers here only
//! render the IR for diagnostics and `eudkit dump-ir`, never mutate it.

use super::{HirModule, HirStmt, HirTerminator, InlineHint, ReactiveRule};
use std::fmt::Write;

fn dump_stmt(out: &mut String, stmt: &HirStmt, indent: usize) {
    let pad = " ".repeat(indent);
    match stmt {
        HirStmt::Trigger {
            label,
            conditions,
            actions,
            preserved,
        } => {
            let lbl = label.as_deref().unwrap_or("anon");
            let _ = writeln!(out, "{pad}trigger @{lbl} (preserved={preserved}) {{");
            for c in conditions {
                let _ = writeln!(out, "{pad}  cond: {c}");
            }
            for a in actions {
                let _ = writeln!(out, "{pad}  act:  {a}");
            }
            let _ = writeln!(out, "{pad}}}");
        }
        HirStmt::Action(act) => {
            let _ = writeln!(out, "{pad}act: {act}");
        }
        HirStmt::VarAssignConst { var, value } => {
            let _ = writeln!(out, "{pad}@v{} = {value}", var.0);
        }
        HirStmt::VarAddConst { var, value } => {
            let _ = writeln!(out, "{pad}@v{} += {value}", var.0);
        }
        HirStmt::VarSubConst { var, value } => {
            let _ = writeln!(out, "{pad}@v{} -= {value}", var.0);
        }
        HirStmt::VarSetMaskConst {
            var,
            modifier,
            value,
            mask,
        } => {
            let _ = writeln!(
                out,
                "{pad}@v{} [mask={mask:#010X}] {modifier} {value}",
                var.0
            );
        }
        HirStmt::VarAssignExpr { var, expr } => {
            let _ = writeln!(out, "{pad}@v{} = {expr}", var.0);
        }
        HirStmt::If {
            conditions,
            then_body,
            else_body,
        } => {
            let cond_strs: Vec<String> = conditions.iter().map(ToString::to_string).collect();
            let _ = writeln!(out, "{pad}if [{}] {{", cond_strs.join(", "));
            for s in then_body {
                dump_stmt(out, s, indent + 2);
            }
            if else_body.is_empty() {
                let _ = writeln!(out, "{pad}}}");
            } else {
                let _ = writeln!(out, "{pad}}} else {{");
                for s in else_body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
        }
        HirStmt::While { conditions, body } => {
            let cond_strs: Vec<String> = conditions.iter().map(ToString::to_string).collect();
            let _ = writeln!(out, "{pad}while [{}] {{", cond_strs.join(", "));
            for s in body {
                dump_stmt(out, s, indent + 2);
            }
            let _ = writeln!(out, "{pad}}}");
        }
        HirStmt::Break => {
            let _ = writeln!(out, "{pad}break");
        }
        HirStmt::Continue => {
            let _ = writeln!(out, "{pad}continue");
        }
        HirStmt::Call { callee, args, rets } => {
            let arg_strs: Vec<String> = args.iter().map(ToString::to_string).collect();
            let ret_strs: Vec<String> = rets.iter().map(|r| format!("@v{}", r.0)).collect();
            if ret_strs.is_empty() {
                let _ = writeln!(out, "{pad}call @fn{}({})", callee.0, arg_strs.join(", "));
            } else {
                let _ = writeln!(
                    out,
                    "{pad}[{}] = call @fn{}({})",
                    ret_strs.join(", "),
                    callee.0,
                    arg_strs.join(", ")
                );
            }
        }
        HirStmt::ReturnValues(values) => {
            let val_strs: Vec<String> = values.iter().map(ToString::to_string).collect();
            let _ = writeln!(out, "{pad}return [{}]", val_strs.join(", "));
        }
        HirStmt::FuncPtrAssign { ptr, target } => {
            let _ = writeln!(out, "{pad}@fp{} = @fn{}", ptr.0, target.0);
        }
        HirStmt::CallIndirect { ptr, args, rets } => {
            let arg_strs: Vec<String> = args.iter().map(ToString::to_string).collect();
            let ret_strs: Vec<String> = rets.iter().map(|r| format!("@v{}", r.0)).collect();
            if ret_strs.is_empty() {
                let _ = writeln!(
                    out,
                    "{pad}call_indirect @fp{}({})",
                    ptr.0,
                    arg_strs.join(", ")
                );
            } else {
                let _ = writeln!(
                    out,
                    "{pad}[{}] = call_indirect @fp{}({})",
                    ret_strs.join(", "),
                    ptr.0,
                    arg_strs.join(", ")
                );
            }
        }
        HirStmt::MemRead {
            dst,
            epd,
            mask,
            shift,
            initial_val,
        } => {
            let _ = writeln!(
                    out,
                    "{pad}@v{} = mem_read(epd={epd}, mask={mask:#010X}, shift={shift}, init={initial_val})",
                    dst.0
                );
        }
        HirStmt::Reactive(rule) => match rule {
            ReactiveRule::Once { cond, body } => {
                let _ = writeln!(out, "{pad}once {{");
                for c in cond {
                    let _ = writeln!(out, "{pad}  cond: {c}");
                }
                for s in body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
            ReactiveRule::When { cond, body } => {
                let _ = writeln!(out, "{pad}when {{");
                for c in cond {
                    let _ = writeln!(out, "{pad}  cond: {c}");
                }
                for s in body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
            ReactiveRule::Until { cond, body } => {
                let _ = writeln!(out, "{pad}until {{");
                for c in cond {
                    let _ = writeln!(out, "{pad}  cond: {c}");
                }
                for s in body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
            ReactiveRule::After { delay_ticks, body } => {
                let _ = writeln!(out, "{pad}after ({delay_ticks} loops) {{");
                for s in body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
            ReactiveRule::Every { period_ticks, body } => {
                let _ = writeln!(out, "{pad}every ({period_ticks} loops) {{");
                for s in body {
                    dump_stmt(out, s, indent + 2);
                }
                let _ = writeln!(out, "{pad}}}");
            }
        },
    }
}

impl HirModule {
    /// Renders a deterministic, human-readable presentation of this HIR
    /// module for diagnostics and `dump-ir`.
    pub fn dump_hir(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "module @{} {{", self.name);
        for var in &self.vars {
            let _ = writeln!(
                out,
                "  var @{} (id=v{}) = {}",
                var.name, var.id.0, var.init_val
            );
        }
        for buf in &self.byte_bufs {
            let _ = writeln!(
                out,
                "  bytebuf @{} (id=buf{}, len={}) = {:02X?}",
                buf.name,
                buf.id.0,
                buf.data.len(),
                buf.data
            );
        }
        for fp in &self.func_ptrs {
            let init_str = fp
                .initial_target
                .map(|t| format!(" = @fn{}", t.0))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "  funcptr @{} (id=fp{}, sig=({}, {})){}",
                fp.name, fp.id.0, fp.signature.0, fp.signature.1, init_str
            );
        }
        for func in &self.functions {
            let role = if func.is_main_loop {
                " (main_loop)"
            } else {
                ""
            };
            if func.params.is_empty()
                && func.return_count == 0
                && func.inline_hint == InlineHint::Auto
            {
                let _ = writeln!(out, "  fn @{}{} {{", func.name, role);
            } else {
                let param_strs: Vec<String> =
                    func.params.iter().map(|p| format!("@v{}", p.0)).collect();
                let _ = writeln!(
                    out,
                    "  fn @{}({}) -> {} rets (hint={:?}){} {{",
                    func.name,
                    param_strs.join(", "),
                    func.return_count,
                    func.inline_hint,
                    role
                );
            }
            for block in &func.blocks {
                let _ = writeln!(out, "    bb{} (@{}):", block.id.0, block.label);
                for stmt in &block.stmts {
                    dump_stmt(&mut out, stmt, 6);
                }
                match &block.terminator {
                    HirTerminator::Jump(target) => {
                        let _ = writeln!(out, "      jump -> bb{}", target.0);
                    }
                    HirTerminator::Branch {
                        conditions,
                        then_block,
                        else_block,
                    } => {
                        let cond_strs: Vec<String> =
                            conditions.iter().map(ToString::to_string).collect();
                        let _ = writeln!(
                            out,
                            "      branch [{}] -> then bb{}, else bb{}",
                            cond_strs.join(", "),
                            then_block.0,
                            else_block.0
                        );
                    }
                    HirTerminator::Return => {
                        let _ = writeln!(out, "      return");
                    }
                }
            }
            let _ = writeln!(out, "  }}");
        }
        let _ = writeln!(out, "}}");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::HirModule;
    use crate::hir::{
        BlockId, Comparison, HirBasicBlock, HirByteBuf, HirExpr, HirFunction, HirStmt,
        HirTerminator, InlineHint, ReactiveRule, StockAction, StockCondition,
    };

    #[test]
    fn dump_hir_renders_module_structure_stably() {
        let mut module = HirModule::new("demo");
        let score = module.add_var("score", 0);
        module.add_byte_buf(HirByteBuf::new("data", vec![0xDE, 0xAD]));
        module.add_func_ptr("handler", (1, 0), None);

        let mut func = HirFunction::new_main_loop("main");
        let entry = func.add_block(HirBasicBlock::new("entry"));
        let block = func.block_mut(entry);
        block.push_stmt(HirStmt::VarAssignConst {
            var: score,
            value: 7,
        });
        block.push_stmt(HirStmt::If {
            conditions: vec![StockCondition::VarCmp {
                var: score,
                comparison: Comparison::Exactly,
                value: 7,
                mask: None,
            }],
            then_body: vec![HirStmt::Action(StockAction::PreserveTrigger)],
            else_body: vec![HirStmt::Continue],
        });
        block.set_terminator(HirTerminator::Jump(BlockId(0)));
        module.add_function(func);

        assert_eq!(
            module.dump_hir(),
            "\
module @demo {
  var @score (id=v0) = 0
  bytebuf @data (id=buf0, len=2) = [DE, AD]
  funcptr @handler (id=fp0, sig=(1, 0))
  fn @main (main_loop) {
    bb0 (@entry):
      @v0 = 7
      if [VarCmp(@v0, Exactly, 7)] {
        act: PreserveTrigger()
      } else {
        continue
      }
      jump -> bb0
  }
}
"
        );
    }

    #[test]
    fn dump_hir_renders_reactive_rules_and_control_flow_stably() {
        let mut module = HirModule::new("reactive");
        let guard = module.add_var("guard", 1);

        let mut func = HirFunction::new("on_tick", false)
            .with_params([guard])
            .with_return_count(2)
            .with_inline_hint(InlineHint::Always);
        let entry = func.add_block(HirBasicBlock::new("entry"));
        let done = func.add_block(HirBasicBlock::new("done"));

        func.block_mut(entry)
            .push_stmt(HirStmt::Reactive(ReactiveRule::Once {
                cond: vec![StockCondition::VarCmp {
                    var: guard,
                    comparison: Comparison::Exactly,
                    value: 1,
                    mask: None,
                }],
                body: vec![HirStmt::VarAssignConst {
                    var: guard,
                    value: 0,
                }],
            }));
        func.block_mut(entry)
            .push_stmt(HirStmt::Reactive(ReactiveRule::After {
                delay_ticks: 3,
                body: vec![HirStmt::Break],
            }));
        func.block_mut(entry).set_terminator(HirTerminator::Branch {
            conditions: vec![StockCondition::Always],
            then_block: done,
            else_block: entry,
        });
        func.block_mut(done).set_terminator(HirTerminator::Return);
        module.add_function(func);

        assert_eq!(
            module.dump_hir(),
            "\
module @reactive {
  var @guard (id=v0) = 1
  fn @on_tick(@v0) -> 2 rets (hint=Always) {
    bb0 (@entry):
      once {
        cond: VarCmp(@v0, Exactly, 1)
        @v0 = 0
      }
      after (3 loops) {
        break
      }
      branch [Always()] -> then bb1, else bb0
    bb1 (@done):
      return
  }
}
"
        );
    }

    #[test]
    fn dump_hir_renders_calls_triggers_and_memory_reads_stably() {
        let mut module = HirModule::new("ops");
        let dst = module.add_var("dst", 0);

        let mut callee = HirFunction::new("callee", false).with_return_count(1);
        callee.add_block(HirBasicBlock::new("entry"));
        let callee_id = module.add_function(callee);

        let mut main = HirFunction::new_main_loop("main");
        let entry = main.add_block(HirBasicBlock::new("entry"));
        main.block_mut(entry).push_stmt(HirStmt::Trigger {
            label: None,
            conditions: vec![StockCondition::Always],
            actions: vec![StockAction::DisplayText {
                text: "hi".to_string(),
                always_display: false,
            }],
            preserved: false,
        });
        main.block_mut(entry).push_stmt(HirStmt::Call {
            callee: callee_id,
            args: vec![HirExpr::Const(1)],
            rets: vec![dst],
        });
        main.block_mut(entry).push_stmt(HirStmt::MemRead {
            dst,
            epd: HirExpr::Const(4),
            mask: 0xFF,
            shift: 0,
            initial_val: 0,
        });
        module.add_function(main);

        assert_eq!(
            module.dump_hir(),
            "\
module @ops {
  var @dst (id=v0) = 0
  fn @callee() -> 1 rets (hint=Auto) {
    bb0 (@entry):
      return
  }
  fn @main (main_loop) {
    bb0 (@entry):
      trigger @anon (preserved=false) {
        cond: Always()
        act:  DisplayText(\"hi\", always=false)
      }
      [@v0] = call @fn0(1)
      @v0 = mem_read(epd=4, mask=0x000000FF, shift=0, init=0)
      return
  }
}
"
        );
    }
}
