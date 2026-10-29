use eudkit_core::config::ProjectConfig;
use eudkit_core::hir::{
    Comparison, HirBasicBlock, HirByteBuf, HirFunction, HirModule, HirStmt, HirTerminator,
    HirValue, Modifier, PlayerId, StockAction, StockCondition,
};
use eudkit_core::lir::{ActField, FieldRef};
use eudkit_core::plan::{HirPlan, PlanMap};
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
    main_fn
        .block_mut(entry_bb)
        .set_terminator(HirTerminator::Branch {
            conditions: vec![StockCondition::Deaths {
                player: PlayerId::Player1,
                comparison: Comparison::AtLeast,
                amount: 0,
                unit: 0,
            }],
            then_block: body_bb,
            else_block: exit_bb,
        });

    main_fn
        .block_mut(body_bb)
        .set_terminator(HirTerminator::Jump(exit_bb));
    main_fn
        .block_mut(exit_bb)
        .set_terminator(HirTerminator::Return);

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
                dts_out = Some(PathBuf::from(args.get(i).ok_or("missing value for --dts")?));
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

fn find_python_bin() -> String {
    if let Ok(py) = std::env::var("PYTHON") {
        if !py.is_empty() {
            return py;
        }
    }
    if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
        let candidate = PathBuf::from(venv).join("bin").join("python");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    // Walk up from current working directory
    let mut cur = std::env::current_dir().ok();
    while let Some(dir) = cur {
        let venv_py = dir.join(".venv").join("bin").join("python");
        if venv_py.exists() {
            return venv_py.to_string_lossy().to_string();
        }
        cur = dir.parent().map(|p| p.to_path_buf());
    }
    // Walk up from current exe directory
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(|p| p.to_path_buf());
        while let Some(dir) = cur {
            let venv_py = dir.join(".venv").join("bin").join("python");
            if venv_py.exists() {
                return venv_py.to_string_lossy().to_string();
            }
            cur = dir.parent().map(|p| p.to_path_buf());
        }
    }
    "python3".to_string()
}

fn configure_python_cmd(cmd: &mut std::process::Command) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(|p| p.to_path_buf());
        while let Some(dir) = cur {
            let src_dir = dir.join("src");
            if src_dir.join("eudkit").join("__init__.py").exists() {
                let existing = std::env::var("PYTHONPATH").unwrap_or_default();
                let new_path = if existing.is_empty() {
                    src_dir.to_string_lossy().to_string()
                } else {
                    format!("{}:{}", src_dir.to_string_lossy(), existing)
                };
                cmd.env("PYTHONPATH", new_path);
                break;
            }
            cur = dir.parent().map(|p| p.to_path_buf());
        }
    }
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
                map_path = Some(PathBuf::from(args.get(i).ok_or("missing value for --map")?));
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
        (
            mp,
            CompilationConfig::default(),
            "tracer_module".to_string(),
            entry_path,
        )
    };

    if let Some(entry) = &resolved_entry {
        let ext = entry.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "py" {
            let py_bin = find_python_bin();
            let mut cmd = std::process::Command::new(&py_bin);
            configure_python_cmd(&mut cmd);
            cmd.arg("-m")
                .arg("eudkit.runner")
                .arg("--input")
                .arg(&resolved_map)
                .arg("--entry")
                .arg(entry);
            if hir_only {
                cmd.arg("--dump-hir");
            } else if lir_only {
                cmd.arg("--dump-lir");
            } else {
                cmd.arg("--dump-hir");
                cmd.arg("--dump-lir");
            }
            let output = cmd
                .output()
                .map_err(|e| format!("failed to execute python runner ({py_bin}): {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Python dump-ir failed:\n{stderr}"));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(out) = out_path {
                fs::write(&out, stdout.as_bytes()).map_err(|e| e.to_string())?;
            } else {
                print!("{stdout}");
            }
            return Ok(());
        }
    }

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

fn execute_python_build(
    proj_cfg: &ProjectConfig,
    entry_path: &Path,
) -> Result<eudkit_core::CompilationReport, String> {
    let py_bin = find_python_bin();
    let mut cmd = std::process::Command::new(&py_bin);
    configure_python_cmd(&mut cmd);
    cmd.arg("-m")
        .arg("eudkit.runner")
        .arg("--input")
        .arg(&proj_cfg.input_map)
        .arg("--output")
        .arg(&proj_cfg.output_map)
        .arg("--entry")
        .arg(entry_path);

    if proj_cfg.obscurate {
        cmd.arg("--obscurate");
    }
    if let Some(seed) = proj_cfg.obscurate_seed {
        cmd.arg("--seed").arg(seed.to_string());
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to execute python runner ({py_bin}): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Python build failed:\n{stderr}\n{stdout}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");

    let session =
        CompilationSession::from_map(&proj_cfg.output_map, proj_cfg.to_compilation_config())
            .map_err(|e| {
                format!(
                    "failed to read generated map {}: {e}",
                    proj_cfg.output_map.display()
                )
            })?;

    Ok(eudkit_core::CompilationReport {
        trigger_count: session.map_context().triggers().len(),
        byte_buf_count: 0,
        payload_offset: 0,
        payload_size: 0,
        payload_entry_addr: 0,
        prt_offsets: Vec::new(),
        ort_offsets: Vec::new(),
    })
}

fn plan_for_session(session: &CompilationSession, module: HirModule) -> HirPlan {
    let mut map = PlanMap::new(session.map_fingerprint());
    if let Some(name) = session
        .input_map_path()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
    {
        map = map.with_name(name);
    }
    HirPlan::new(module).with_map(map)
}

fn read_plan(path: &Path) -> Result<HirPlan, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("failed to read HIR plan {}: {e}", path.display()))?;
    HirPlan::from_json(&text).map_err(|e| e.to_string())
}

fn write_plan(path: &Path, plan: &HirPlan) -> Result<(), String> {
    let json = plan.to_json().map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(path, &json)
        .map_err(|e| format!("failed to write HIR plan {}: {e}", path.display()))?;
    println!("Wrote HIR plan {} ({} bytes)", path.display(), json.len());
    Ok(())
}

fn run_python_plan(
    proj_cfg: &ProjectConfig,
    entry_path: &Path,
    plan_path: &Path,
) -> Result<(), String> {
    let py_bin = find_python_bin();
    let mut cmd = std::process::Command::new(&py_bin);
    configure_python_cmd(&mut cmd);
    cmd.arg("-m")
        .arg("eudkit.runner")
        .arg("--input")
        .arg(&proj_cfg.input_map)
        .arg("--entry")
        .arg(entry_path)
        .arg("--plan")
        .arg(plan_path);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to execute python runner ({py_bin}): {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Python synthesis failed:\n{stderr}\n{stdout}"));
    }
    if !plan_path.exists() {
        return Err(format!(
            "Python runner reported success but did not write {}",
            plan_path.display()
        ));
    }
    Ok(())
}

fn synth_ts_plan(proj_cfg: &ProjectConfig, entry_path: &Path) -> Result<HirPlan, String> {
    let session =
        CompilationSession::from_map(&proj_cfg.input_map, proj_cfg.to_compilation_config())
            .map_err(|e| e.to_string())?;
    let hir = eudkit_ts::compile_ts_file(entry_path, &proj_cfg.name, session.map_context())
        .map_err(|e| e.to_string())?;
    Ok(plan_for_session(&session, hir))
}

fn temporary_plan_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("eudkit-plan-{}-{nanos}.eudhir", std::process::id()))
}

fn cmd_synth(args: &[String]) -> Result<(), String> {
    let mut config_path: Option<PathBuf> = None;
    let mut input_map: Option<PathBuf> = None;
    let mut entry_path: Option<PathBuf> = None;
    let mut plan_path: Option<PathBuf> = None;
    let mut check = false;

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
            "--entry" | "-e" => {
                i += 1;
                entry_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --entry")?,
                ));
            }
            "--plan" | "-p" | "--output" | "-o" => {
                i += 1;
                plan_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --plan")?,
                ));
            }
            "--check" => check = true,
            "--help" | "-h" => {
                println!("usage: eudkit synth [--config <eudkit.toml>] [--input <in.scx>] [--entry <script.(ts|py)>] [--plan <out.eudhir>] [--check]");
                return Ok(());
            }
            other => return Err(format!("unknown argument for synth: {other}")),
        }
        i += 1;
    }

    let mut proj_cfg = if let Some(cfg_file) = config_path {
        ProjectConfig::from_file(cfg_file).map_err(|e| e.to_string())?
    } else if Path::new("eudkit.toml").exists() && (input_map.is_none() || entry_path.is_none()) {
        ProjectConfig::from_file("eudkit.toml").map_err(|e| e.to_string())?
    } else {
        ProjectConfig::default()
    };

    if let Some(input) = input_map {
        proj_cfg.input_map = input;
    }
    if let Some(entry) = entry_path {
        proj_cfg.entry = Some(entry);
    }

    if proj_cfg.input_map.as_os_str().is_empty() {
        return Err(
            "usage: eudkit synth --input <in.scx> --entry <script.(ts|py)> --plan <out.eudhir>"
                .to_string(),
        );
    }
    let entry = proj_cfg.entry.clone().ok_or(
        "usage: eudkit synth --input <in.scx> --entry <script.(ts|py)> --plan <out.eudhir>",
    )?;
    let plan_path = plan_path.unwrap_or_else(|| {
        if proj_cfg.output_map.as_os_str().is_empty() {
            PathBuf::from("plan.eudhir")
        } else {
            proj_cfg.output_map.with_extension("eudhir")
        }
    });

    let ext = entry
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let generated = if ext == "py" {
        let generated_path = if check {
            temporary_plan_path()
        } else {
            plan_path.clone()
        };
        let result = run_python_plan(&proj_cfg, &entry, &generated_path)
            .and_then(|_| read_plan(&generated_path));
        if check {
            let _ = fs::remove_file(&generated_path);
        }
        result?
    } else if ext == "ts" || ext == "js" {
        synth_ts_plan(&proj_cfg, &entry)?
    } else {
        return Err(format!(
            "unsupported entry extension '.{ext}': expected .py, .ts, or .js"
        ));
    };

    if check {
        let existing = read_plan(&plan_path)
            .map_err(|e| format!("cannot check {}: {e}", plan_path.display()))?;
        if existing != generated {
            return Err(format!(
                "HIR plan {} is out of date; run `eudkit synth` and commit the result",
                plan_path.display()
            ));
        }
        println!("HIR plan {} is up to date", plan_path.display());
    } else {
        write_plan(&plan_path, &generated)?;
    }

    Ok(())
}

fn cmd_compile(args: &[String]) -> Result<(), String> {
    let mut config_path: Option<PathBuf> = None;
    let mut input_map: Option<PathBuf> = None;
    let mut output_map: Option<PathBuf> = None;
    let mut plan_path: Option<PathBuf> = None;
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
            "--plan" | "-p" => {
                i += 1;
                plan_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --plan")?,
                ));
            }
            "--obscurate" => obscurate = Some(true),
            "--seed" => {
                i += 1;
                let value = args.get(i).ok_or("missing value for --seed")?;
                seed = Some(value.parse::<u64>().map_err(|e| e.to_string())?);
            }
            "--help" | "-h" => {
                println!("usage: eudkit compile --plan <plan.eudhir> --input <in.scx> --output <out.scx>");
                return Ok(());
            }
            other => return Err(format!("unknown argument for compile: {other}")),
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

    if let Some(input) = input_map {
        proj_cfg.input_map = input;
    }
    if let Some(output) = output_map {
        proj_cfg.output_map = output;
    }
    if let Some(obscurate) = obscurate {
        proj_cfg.obscurate = obscurate;
    }
    if let Some(seed) = seed {
        proj_cfg.obscurate_seed = Some(seed);
    }

    if proj_cfg.input_map.as_os_str().is_empty() || proj_cfg.output_map.as_os_str().is_empty() {
        return Err(
            "usage: eudkit compile --plan <plan.eudhir> --input <in.scx> --output <out.scx>"
                .to_string(),
        );
    }
    let plan_path = plan_path
        .ok_or("usage: eudkit compile --plan <plan.eudhir> --input <in.scx> --output <out.scx>")?;
    let plan = read_plan(&plan_path)?;

    let mut session =
        CompilationSession::from_map(&proj_cfg.input_map, proj_cfg.to_compilation_config())
            .map_err(|e| e.to_string())?;

    if let Some(map) = &plan.map {
        let actual = session.map_fingerprint();
        if map.fingerprint != actual {
            return Err(format!(
                "HIR plan map fingerprint mismatch: plan was synthesized for {}, input map is {}. Re-run `eudkit synth`.",
                map.fingerprint, actual
            ));
        }
    } else {
        eprintln!(
            "warning: HIR plan has no map fingerprint; compiling without a Map Context identity check"
        );
    }

    session.set_hir_module(plan.module);
    let report = session
        .compile_map(&proj_cfg.output_map)
        .map_err(|e| e.to_string())?;
    println!(
        "Compiled {} (triggers={}, payload_bytes={}, entry={:#010X})",
        proj_cfg.output_map.display(),
        report.trigger_count,
        report.payload_size,
        report.payload_entry_addr
    );
    Ok(())
}

fn execute_build(proj_cfg: &ProjectConfig) -> Result<eudkit_core::CompilationReport, String> {
    if proj_cfg.input_map.as_os_str().is_empty() || proj_cfg.output_map.as_os_str().is_empty() {
        return Err(
            "usage: eudkit build [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <script.(ts|py)>]"
                .to_string(),
        );
    }

    if let Some(entry_path) = &proj_cfg.entry {
        let ext = entry_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if ext == "py" {
            return execute_python_build(proj_cfg, entry_path);
        }
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
    Ok(report)
}

fn cmd_watch(args: &[String]) -> Result<(), String> {
    let mut config_path: Option<PathBuf> = None;
    let mut input_map: Option<PathBuf> = None;
    let mut output_map: Option<PathBuf> = None;
    let mut entry_path: Option<PathBuf> = None;
    let mut dts_out: Option<PathBuf> = None;
    let mut obscurate: Option<bool> = None;
    let mut seed: Option<u64> = None;
    let mut once = false;

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
            "--once" => once = true,
            "--watch" | "-w" => {}
            other => return Err(format!("unknown argument for watch: {other}")),
        }
        i += 1;
    }

    let mut proj_cfg = if let Some(cfg_file) = config_path.as_ref() {
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
            "usage: eudkit watch [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <main.ts>] [--once]"
                .to_string(),
        );
    }

    println!("[watch] Starting eudkit watch mode for {}", proj_cfg.name);

    if let Err(e) = execute_build(&proj_cfg) {
        eprintln!("[watch] Initial build error: {e}");
        if once {
            return Err(e);
        }
    } else {
        println!("[watch] Initial build succeeded.");
    }

    if once {
        return Ok(());
    }

    let mut last_modified = std::collections::HashMap::new();

    let collect_watched_files = |cfg: &ProjectConfig, cp: Option<&PathBuf>| -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(c) = cp {
            paths.push(c.clone());
        } else if Path::new("eudkit.toml").exists() {
            paths.push(PathBuf::from("eudkit.toml"));
        }
        paths.push(cfg.input_map.clone());

        if let Some(entry) = &cfg.entry {
            paths.push(entry.clone());
            if let Some(parent) = entry.parent() {
                let dir = if parent.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent
                };
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if let Some(ext) = p.extension() {
                            if ext == "ts" || ext == "js" || ext == "json" {
                                paths.push(p);
                            }
                        }
                    }
                }
            }
        }
        paths.sort();
        paths.dedup();
        paths
    };

    for p in collect_watched_files(&proj_cfg, config_path.as_ref()) {
        if let Ok(meta) = fs::metadata(&p) {
            if let Ok(mtime) = meta.modified() {
                last_modified.insert(p, mtime);
            }
        }
    }

    println!(
        "[watch] Watching {} files for changes. Press Ctrl+C to stop.",
        last_modified.len()
    );

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let current_files = collect_watched_files(&proj_cfg, config_path.as_ref());
        let mut changed_file: Option<PathBuf> = None;

        for p in &current_files {
            if let Ok(meta) = fs::metadata(p) {
                if let Ok(mtime) = meta.modified() {
                    if let Some(&prev_time) = last_modified.get(p) {
                        if mtime > prev_time {
                            changed_file = Some(p.clone());
                            break;
                        }
                    } else {
                        changed_file = Some(p.clone());
                        break;
                    }
                }
            }
        }

        if let Some(changed) = changed_file {
            println!(
                "[watch] Change detected in {}. Rebuilding...",
                changed.display()
            );
            for p in &current_files {
                if let Ok(meta) = fs::metadata(p) {
                    if let Ok(mtime) = meta.modified() {
                        last_modified.insert(p.clone(), mtime);
                    }
                }
            }

            match execute_build(&proj_cfg) {
                Ok(_) => println!("[watch] Rebuild succeeded."),
                Err(e) => eprintln!("[watch] Rebuild error: {e}"),
            }
        }
    }
}

fn cmd_build(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--watch" || a == "-w") {
        return cmd_watch(args);
    }

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
            "--watch" | "-w" => {}
            "--help" | "-h" => {
                println!("usage: eudkit build [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <script.(ts|py)>]");
                return Ok(());
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

    execute_build(&proj_cfg).map(|_| ())
}

fn cmd_migrate(args: &[String]) -> Result<(), String> {
    let mut target_path: Option<PathBuf> = None;
    let mut out_path: Option<PathBuf> = None;
    let mut toml_out: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output" | "-o" => {
                i += 1;
                out_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --output")?,
                ));
            }
            "--config-out" => {
                i += 1;
                toml_out = Some(PathBuf::from(
                    args.get(i).ok_or("missing value for --config-out")?,
                ));
            }
            other if !other.starts_with('-') && target_path.is_none() => {
                target_path = Some(PathBuf::from(other));
            }
            other => return Err(format!("unknown argument for migrate: {other}")),
        }
        i += 1;
    }

    let target = if let Some(p) = target_path {
        p
    } else {
        let mut found: Option<PathBuf> = None;
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("edd") || ext.eq_ignore_ascii_case("eds") {
                        found = Some(path);
                        break;
                    }
                    if ext.eq_ignore_ascii_case("eps") && found.is_none() {
                        found = Some(path);
                    }
                }
            }
        }
        found.ok_or_else(|| {
            "usage: eudkit migrate <project.edd|project.eds|file.eps> [-o output]".to_string()
        })?
    };

    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "edd" | "eds" => {
            let content = fs::read_to_string(&target)
                .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
            let project = eudkit_cli::migrate::ini::parse_edd_or_eds(&content, &target)
                .map_err(|e| format!("failed to parse {}: {e}", target.display()))?;

            let parent_dir = target.parent().unwrap_or_else(|| Path::new("."));

            // 1. Convert entry .eps if present
            if let Some(ref eps_rel) = project.entry_eps {
                let eps_full = if eps_rel.is_relative() && !parent_dir.as_os_str().is_empty() {
                    let cand = parent_dir.join(eps_rel);
                    if cand.exists() {
                        cand
                    } else {
                        eps_rel.clone()
                    }
                } else {
                    eps_rel.clone()
                };

                if eps_full.exists() {
                    let eps_src = fs::read_to_string(&eps_full)
                        .map_err(|e| format!("failed to read {}: {e}", eps_full.display()))?;
                    let ts_code = eudkit_cli::migrate::migrator::convert_eps_to_ts(&eps_src)
                        .map_err(|e| format!("failed to convert {}: {e}", eps_full.display()))?;

                    let ts_out_path = if let Some(ref o) = out_path {
                        if o.is_dir() {
                            o.join(&project.entry_ts)
                        } else {
                            o.clone()
                        }
                    } else if parent_dir.as_os_str().is_empty() {
                        project.entry_ts.clone()
                    } else {
                        parent_dir.join(&project.entry_ts)
                    };

                    if let Some(p) = ts_out_path.parent() {
                        if !p.as_os_str().is_empty() {
                            fs::create_dir_all(p).map_err(|e| e.to_string())?;
                        }
                    }
                    fs::write(&ts_out_path, ts_code)
                        .map_err(|e| format!("failed to write {}: {e}", ts_out_path.display()))?;
                    println!(
                        "Migrated {} -> {}",
                        eps_full.display(),
                        ts_out_path.display()
                    );
                } else {
                    println!(
                        "Warning: entry script {} does not exist on disk",
                        eps_full.display()
                    );
                }
            }

            // 2. Write eudkit.toml
            let toml_path = toml_out.unwrap_or_else(|| {
                if parent_dir.as_os_str().is_empty() {
                    PathBuf::from("eudkit.toml")
                } else {
                    parent_dir.join("eudkit.toml")
                }
            });

            fs::write(&toml_path, project.to_toml_string())
                .map_err(|e| format!("failed to write {}: {e}", toml_path.display()))?;
            println!("Generated {}", toml_path.display());
            Ok(())
        }
        "eps" => {
            let eps_src = fs::read_to_string(&target)
                .map_err(|e| format!("failed to read {}: {e}", target.display()))?;
            let ts_code = eudkit_cli::migrate::migrator::convert_eps_to_ts(&eps_src)
                .map_err(|e| format!("failed to convert {}: {e}", target.display()))?;

            let ts_out_path = if let Some(o) = out_path {
                if o.is_dir() {
                    let mut stem = target.file_stem().unwrap_or_default().to_os_string();
                    stem.push(".ts");
                    o.join(stem)
                } else {
                    o
                }
            } else {
                let mut ts = target.clone();
                ts.set_extension("ts");
                ts
            };

            if let Some(p) = ts_out_path.parent() {
                if !p.as_os_str().is_empty() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            fs::write(&ts_out_path, ts_code)
                .map_err(|e| format!("failed to write {}: {e}", ts_out_path.display()))?;
            println!("Migrated {} -> {}", target.display(), ts_out_path.display());
            Ok(())
        }
        other => Err(format!(
            "unsupported file extension '.{other}': expected .edd, .eds, or .eps"
        )),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("usage: eudkit <inspect|dump-ir|build|synth|compile|watch|migrate> [options]");
        process::exit(1);
    };

    let res = match cmd.as_str() {
        "inspect" => cmd_inspect(rest),
        "dump-ir" => cmd_dump_ir(rest),
        "build" => cmd_build(rest),
        "synth" => cmd_synth(rest),
        "compile" => cmd_compile(rest),
        "watch" => cmd_watch(rest),
        "migrate" => cmd_migrate(rest),
        "--help" | "-h" | "help" => {
            println!("eudkit <inspect|dump-ir|build|synth|compile|watch|migrate>");
            Ok(())
        }
        "--version" | "-v" | "version" => {
            println!("eudkit {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        other => Err(format!("unknown subcommand '{other}'")),
    };

    if let Err(err) = res {
        eprintln!("error: {err}");
        process::exit(1);
    }
}
