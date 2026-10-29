use eudkit_core::config::{BuildRequest, ProjectConfig};
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
    let ctx = session.map_data();

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
        let dts = eudkit_core::dts::generate_dts(ctx, session.canonical_symbols(), mod_name);
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
    let request = BuildRequest::from_args(args).map_err(|e| e.to_string())?;
    if request.help {
        println!(
            "usage: eudkit dump-ir [--config <eudkit.toml>] [--map <map.scx>] [--entry <script>] [--hir-only|--lir-only]"
        );
        return Ok(());
    }

    let resolved = request.resolve().map_err(|e| e.to_string())?;
    if resolved.config.input_map.as_os_str().is_empty() {
        return Err("usage: eudkit dump-ir --map <map.scx>".to_string());
    }

    let resolved_map = resolved.config.input_map.clone();
    let resolved_entry = resolved.config.entry.clone();
    let comp_cfg = resolved.config.to_compilation_config();
    let mod_name = if resolved.config_path.is_some() {
        resolved.config.name.clone()
    } else {
        "tracer_module".to_string()
    };
    let hir_only = request.hir_only;
    let lir_only = request.lir_only;
    let out_path = request.output_map.clone();

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
    // The Python runner writes one typed JSON `CompilationReport` here. This is
    // an implementation detail of the Python Frontend handoff; the Rust CLI
    // never interprets the file as ad-hoc text. A file is used instead of
    // stdout because the entry script may print arbitrary user output there.
    let report_path = env::temp_dir().join(format!("eudkit-python-report-{}.json", process::id()));
    let _ = fs::remove_file(&report_path);

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
        .arg(entry_path)
        .arg("--report")
        .arg(&report_path);

    if proj_cfg.obscurate {
        cmd.arg("--obscurate");
    }
    if let Some(seed) = proj_cfg.obscurate_seed {
        cmd.arg("--seed").arg(seed.to_string());
    }
    if !proj_cfg.preserve_classic_triggers {
        cmd.arg("--no-preserve-classic-triggers");
    }

    let output = cmd
        .output()
        .map_err(|e| format!("failed to execute python runner ({py_bin}): {e}"))?;

    if !output.status.success() {
        let _ = fs::remove_file(&report_path);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Python build failed:\n{stderr}\n{stdout}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");

    let report_text = fs::read_to_string(&report_path).map_err(|e| {
        format!(
            "Python build did not report its result ({e}): {}",
            report_path.display()
        )
    })?;
    let _ = fs::remove_file(&report_path);
    parse_python_report(&report_text)
        .map_err(|e| format!("invalid Python build report {}: {e}", report_path.display()))
}

/// Parses the typed JSON report written by `python -m eudkit.runner --report`.
///
/// Every field is required; a missing key is an error rather than a zeroed
/// value.
fn parse_python_report(text: &str) -> Result<eudkit_core::CompilationReport, String> {
    eudkit_core::CompilationReport::from_json(text).map_err(|e| e.to_string())
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
        let dts = eudkit_core::dts::generate_dts(
            session.map_data(),
            session.canonical_symbols(),
            &proj_cfg.name,
        );
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

    println!("{}", report.summary(proj_cfg.output_map.display()));
    Ok(report)
}

fn cmd_watch(args: &[String]) -> Result<(), String> {
    let request = BuildRequest::from_args(args).map_err(|e| e.to_string())?;
    if request.help {
        println!(
            "usage: eudkit watch [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <main.ts>] [--once]"
        );
        return Ok(());
    }
    run_watch(&request)
}

fn run_watch(request: &BuildRequest) -> Result<(), String> {
    let resolved = request.resolve().map_err(|e| e.to_string())?;
    let proj_cfg = resolved.config;
    let config_path = resolved.config_path;
    let once = request.once;

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
    let request = BuildRequest::from_args(args).map_err(|e| e.to_string())?;
    if request.help {
        println!(
            "usage: eudkit build [--config <eudkit.toml>] [--input <in.scx> --output <out.scx>] [--entry <script.(ts|py)>]"
        );
        return Ok(());
    }
    if request.watch {
        return run_watch(&request);
    }

    let resolved = request.resolve().map_err(|e| e.to_string())?;
    execute_build(&resolved.config).map(|_| ())
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
        eprintln!("usage: eudkit <inspect|dump-ir|build|watch|migrate> [options]");
        process::exit(1);
    };

    let res = match cmd.as_str() {
        "inspect" => cmd_inspect(rest),
        "dump-ir" => cmd_dump_ir(rest),
        "build" => cmd_build(rest),
        "watch" => cmd_watch(rest),
        "migrate" => cmd_migrate(rest),
        "--help" | "-h" | "help" => {
            println!("eudkit <inspect|dump-ir|build|watch|migrate>");
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

#[cfg(test)]
mod tests {
    use super::parse_python_report;
    use eudkit_core::CompilationReport;

    fn sample_report() -> CompilationReport {
        CompilationReport {
            trigger_count: 12,
            byte_buf_count: 3,
            vproc_count: 1,
            payload_offset: 4096,
            payload_size: 512,
            payload_entry_addr: 285_278_210,
            prt_offsets: vec![16, 32],
            ort_offsets: vec![48],
        }
    }

    #[test]
    fn parses_the_typed_json_python_build_report_including_vproc_count() {
        let text = sample_report().to_json().expect("report should serialize");
        let report = parse_python_report(&text).expect("should parse report");
        assert_eq!(report, sample_report());
        assert_eq!(report.vproc_count, 1);
    }

    #[test]
    fn rejects_reports_with_missing_required_fields() {
        let text = sample_report().to_json().expect("report should serialize");

        // `vproc_count` must not silently default to zero when absent.
        let missing_vproc = text.replace("\"vproc_count\":1,", "");
        assert_ne!(missing_vproc, text);
        assert!(parse_python_report(&missing_vproc).is_err());

        let missing_payload = text.replace("\"payload_size\":512,", "");
        assert_ne!(missing_payload, text);
        assert!(parse_python_report(&missing_payload).is_err());

        // The old `key=value` format is no longer accepted.
        assert!(parse_python_report("trigger_count=12\n").is_err());
    }

    #[test]
    fn renders_the_shared_build_summary() {
        let report = sample_report();
        assert_eq!(
            report.summary("out.scx"),
            "Built out.scx (triggers=12, payload_bytes=512, entry=0x11010002)"
        );
    }
}
