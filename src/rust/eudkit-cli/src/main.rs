use eudkit_core::config::ProjectConfig;
use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirByteBuf, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, PlayerId, StockAction, StockCondition,
};
use eudkit_core::lir::{ActField, FieldRef};
use eudkit_core::{CompilationConfig, CompilationSession};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn default_tracer_hir(module_name: &str) -> HirModule {
    let mut hir = HirModule::new(module_name);
    let tick_buf = hir.add_byte_buf(HirByteBuf::new("tick_counter", vec![0, 0, 0, 0]));

    let mut main_fn = HirFunction::new_main_loop("main");
    let entry_bb = main_fn.add_block(HirBasicBlock::new("entry"));
    let body_bb = main_fn.add_block(HirBasicBlock::new("tick_step"));
    let exit_bb = main_fn.add_block(HirBasicBlock::new("exit"));

    let act_val_ref = FieldRef::action(0, 0, ActField::Player2);

    main_fn.block_mut(entry_bb).push_stmt(HirStmt::Trigger {
        label: Some("main_tick".to_string()),
        conditions: vec![StockCondition::Always],
        actions: vec![
            StockAction::SetDeaths {
                player: PlayerId::Player1,
                modifier: Modifier::Add,
                value: HirValue::Const(0),
                unit: 0,
            },
            StockAction::SetField {
                target: act_val_ref,
                modifier: Modifier::SetTo,
                value: HirValue::ByteBufAddress(tick_buf, 0),
            },
        ],
        preserved: true,
    });
    main_fn.block_mut(entry_bb).set_terminator(HirTerminator::Branch {
        conditions: vec![StockCondition::Deaths {
            player: PlayerId::Player1,
            comparison: Comparison::AtLeast,
            amount: 0,
            unit: 0,
        }],
        then_block: body_bb,
        else_block: exit_bb,
    });

    main_fn.block_mut(body_bb).set_terminator(HirTerminator::Jump(exit_bb));
    main_fn.block_mut(exit_bb).set_terminator(HirTerminator::Return);

    hir.add_function(main_fn);
    hir
}

fn cmd_inspect(args: &[String]) -> Result<(), String> {
    let mut map_arg: Option<PathBuf> = None;
    let mut dts_out: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dts" => {
                i += 1;
                dts_out = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --dts")?,
                ));
            }
            other if !other.starts_with('-') && map_arg.is_none() => {
                map_arg = Some(PathBuf::from(other));
            }
            other => return Err(format!("unknown argument for inspect: {other}")),
        }
        i += 1;
    }

    let map_path = map_arg
        .ok_or_else(|| "usage: eudkit inspect <map.scx> [--dts <output.d.ts>]".to_string())?;
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .map_err(|e| e.to_string())?;
    let ctx = session.map_context();

    let named_locations = ctx.locations().iter().filter(|l| l.name.is_some()).count();
    let cuwp_used = ctx.cuwp_slots().iter().filter(|s| s.in_use).count();
    let custom_units = ctx
        .unit_settings()
        .iter()
        .filter(|u| !u.use_default || u.custom_name.is_some())
        .count();
    let named_switches = ctx.switches().iter().filter(|s| s.name.is_some()).count();
    let active_players = ctx.players()[..8].iter().filter(|p| p.owner != 0).count();

    println!("Map Context Inspection: {}", map_path.display());
    println!("  Strings:      {}", ctx.strings().len());
    println!(
        "  Locations:    {} total ({} named, Anywhere=64)",
        ctx.locations().len(),
        named_locations
    );
    println!("  Placed Units: {}", ctx.placed_units().len());
    println!(
        "  CUWP Slots:   {} total ({} in use)",
        ctx.cuwp_slots().len(),
        cuwp_used
    );
    println!(
        "  Unit Settings: {} total ({} customized)",
        ctx.unit_settings().len(),
        custom_units
    );
    println!(
        "  Switches:     {} total ({} named)",
        ctx.switches().len(),
        named_switches
    );
    println!(
        "  Players:      {} slots ({} active in P1..P8)",
        ctx.players().len(),
        active_players
    );
    println!("  TRIG Triggers: {}", ctx.triggers().len());

    if let Some(dts_path) = dts_out {
        let mod_name = map_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("map");
        let dts = ctx.generate_dts(mod_name);
        if let Some(parent) = dts_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        fs::write(&dts_path, dts).map_err(|e| e.to_string())?;
        println!("  Generated .d.ts: {}", dts_path.display());
    }

    Ok(())
}

fn cmd_dump_ir(args: &[String]) -> Result<(), String> {
    let mut map_path: Option<PathBuf> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut entry_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut hir_only = false;
    let mut lir_only = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--map" | "-m" | "--input" | "-i" => {
                i += 1;
                map_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --map")?,
                ));
            }
            "--config" | "-c" => {
                i += 1;
                config_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --config")?,
                ));
            }
            "--entry" | "-e" => {
                i += 1;
                entry_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --entry")?,
                ));
            }
            "--output" | "-o" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --output")?,
                ));
            }
            "--hir-only" => hir_only = true,
            "--lir-only" => lir_only = true,
            other if !other.starts_with('-') && map_path.is_none() => {
                map_path = Some(PathBuf::from(other));
            }
            other => return Err(format!("unknown argument for dump-ir: {other}")),
        }
        i += 1;
    }

    let (resolved_map, comp_cfg, mod_name, resolved_entry) = if let Some(cfg_file) = config_path {
        let pcfg = ProjectConfig::from_file(&cfg_file).map_err(|e| e.to_string())?;
        (
            map_path.unwrap_or(pcfg.input_map.clone()),
            pcfg.to_compilation_config(),
            pcfg.name,
            entry_path.or(pcfg.entry),
        )
    } else if Path::new("eudkit.toml").exists() && map_path.is_none() {
        let pcfg = ProjectConfig::from_file("eudkit.toml").map_err(|e| e.to_string())?;
        (
            pcfg.input_map.clone(),
            pcfg.to_compilation_config(),
            pcfg.name,
            entry_path.or(pcfg.entry),
        )
    } else {
        let mp = map_path.ok_or("usage: eudkit dump-ir --map <map.scx>")?;
        (mp, CompilationConfig::default(), "tracer_module".to_string(), entry_path)
    };

    let mut session =
        CompilationSession::from_map(&resolved_map, comp_cfg).map_err(|e| e.to_string())?;

    if let Some(entry) = resolved_entry {
        let hir = eudkit_ts::compile_ts_file(&entry, &mod_name, session.map_context())
            .map_err(|e| e.to_string())?;
        session.set_hir_module(hir);
    } else {
        session.set_hir_module(default_tracer_hir(&mod_name));
    }

    let mut combined = String::new();
    if !lir_only {
        combined.push_str(&session.dump_hir());
    }
    if !hir_only {
        if !combined.is_empty() {
            combined.push('\n');
        }
        let lir_text = session.dump_lir().map_err(|e| e.to_string())?;
        combined.push_str(&lir_text);
    }

    if let Some(out) = out_path {
        fs::write(&out, &combined).map_err(|e| e.to_string())?;
    } else {
        print!("{combined}");
    }
    Ok(())
}

fn cmd_build(args: &[String]) -> Result<(), String> {
    let mut config_path: Option<PathBuf> = None;
    let mut input_map: Option<PathBuf> = None;
    let mut output_map: Option<PathBuf> = None;
    let mut entry_path: Option<PathBuf> = None;
    let mut dts_out: Option<PathBuf> = None;
    let mut obscurate: Option<bool> = None;
    let mut seed: Option<u64> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                i += 1;
                config_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --config")?,
                ));
            }
            "--input" | "-i" => {
                i += 1;
                input_map = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --input")?,
                ));
            }
            "--output" | "-o" => {
                i += 1;
                output_map = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --output")?,
                ));
            }
            "--entry" | "-e" => {
                i += 1;
                entry_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --entry")?,
                ));
            }
            "--dts" => {
                i += 1;
                dts_out = Some(PathBuf::from(args.get(i).ok_or("missing value for --dts")?));
            }
            "--obscurate" => obscurate = Some(true),
            "--seed" => {
                i += 1;
                let s = args.get(i).ok_or("missing value for --seed")?;
                seed = Some(s.parse::<u64>().map_err(|e| e.to_string())?);
            }
            other => return Err(format!("unknown argument for build: {other}")),
        }
        i += 1;
    }

    let mut proj_cfg = if let Some(cfg_file) = config_path {
        ProjectConfig::from_file(cfg_file).map_err(|e| e.to_string())?
    } else if Path::new("eudkit.toml").exists() && (input_map.is_none() || output_map.is_none()) {
        ProjectConfig::from_file("eudkit.toml").map_err(|e| e.to_string())?
    } else {
        ProjectConfig::default()
    };

    if let Some(inp) = input_map {
        proj_cfg.input_map = inp;
    }
    if let Some(out) = output_map {
        proj_cfg.output_map = out;
    }
    if let Some(ent) = entry_path {
        proj_cfg.entry = Some(ent);
    }
    if let Some(dts) = dts_out {
        proj_cfg.dts = Some(dts);
    }
    if let Some(obs) = obscurate {
        proj_cfg.obscurate = obs;
    }
    if let Some(sd) = seed {
        proj_cfg.obscurate_seed = Some(sd);
    }

    if proj_cfg.input_map.as_os_str().is_empty() || proj_cfg.output_map.as_os_str().is_empty() {
        return Err(
            "usage: eudkit build [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <main.ts>]"
                .to_string(),
        );
    }

    let mut session =
        CompilationSession::from_map(&proj_cfg.input_map, proj_cfg.to_compilation_config())
            .map_err(|e| e.to_string())?;

    if let Some(dts_path) = &proj_cfg.dts {
        let dts = session.map_context().generate_dts(&proj_cfg.name);
        if let Some(parent) = dts_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        fs::write(dts_path, dts).map_err(|e| e.to_string())?;
    }

    if let Some(entry_path) = &proj_cfg.entry {
        let hir = eudkit_ts::compile_ts_file(entry_path, &proj_cfg.name, session.map_context())
            .map_err(|e| e.to_string())?;
        session.set_hir_module(hir);
    } else {
        session.set_hir_module(default_tracer_hir(&proj_cfg.name));
    }

    let report = session
        .compile_map(&proj_cfg.output_map)
        .map_err(|e| e.to_string())?;

    println!(
        "Built {} (triggers={}, payload_bytes={}, entry={:#010X})",
        proj_cfg.output_map.display(),
        report.trigger_count,
        report.payload_size,
        report.payload_entry_addr
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("usage: eudkit <inspect|dump-ir|build> [options]");
        process::exit(1);
    };

    let res = match cmd.as_str() {
        "inspect" => cmd_inspect(rest),
        "dump-ir" => cmd_dump_ir(rest),
        "build" => cmd_build(rest),
        "--help" | "-h" | "help" => {
            println!("eudkit <inspect|dump-ir|build>");
            Ok(())
        }
        other => Err(format!("unknown subcommand '{other}'")),
    };

    if let Err(err) = res {
        eprintln!("error: {err}");
        process::exit(1);
    }
}
