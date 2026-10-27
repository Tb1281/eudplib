use eudkit_cli::migrate::ini::parse_edd_or_eds;
use eudkit_cli::migrate::migrator::convert_eps_to_ts;
use eudkit_core::chk::Chk;
use eudkit_core::{CompilationConfig, CompilationSession};
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn eudkit_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_eudkit"))
}

#[test]
fn test_parse_edd_ini_format() {
    let edd_content = r#"
# euddraft configuration
[main]
input: tests/unittests/testmap.scx
output: build/migrated_out.scx

[eudTurbo]
[freezeFilter]

[src/main.eps]
"#;

    let project = parse_edd_or_eds(edd_content, Path::new("project.edd")).expect("should parse .edd");
    assert_eq!(project.input_map, PathBuf::from("tests/unittests/testmap.scx"));
    assert_eq!(project.output_map, PathBuf::from("build/migrated_out.scx"));
    assert_eq!(project.entry_eps, Some(PathBuf::from("src/main.eps")));
    assert_eq!(project.entry_ts, PathBuf::from("src/main.ts"));
    assert_eq!(project.name, "main");

    let toml = project.to_toml_string();
    assert!(toml.contains("[project]"));
    assert!(toml.contains("name = \"main\""));
    assert!(toml.contains("entry = \"src/main.ts\""));
    assert!(toml.contains("[build]"));
    assert!(toml.contains("input_map = \"tests/unittests/testmap.scx\""));
    assert!(toml.contains("output_map = \"build/migrated_out.scx\""));
    assert!(toml.contains("obscurate = false"));
}

#[test]
fn test_parse_eds_ini_format_with_equals() {
    let eds_content = r#"
[main]
input = basemap.scx
output = output.scx
entry = game.eps

[plugins]
unpatch = 1
"#;

    let project = parse_edd_or_eds(eds_content, Path::new("map.eds")).expect("should parse .eds");
    assert_eq!(project.input_map, PathBuf::from("basemap.scx"));
    assert_eq!(project.output_map, PathBuf::from("output.scx"));
    assert_eq!(project.entry_eps, Some(PathBuf::from("game.eps")));
    assert_eq!(project.entry_ts, PathBuf::from("game.ts"));
    assert_eq!(project.name, "game");
}

#[test]
fn test_convert_variables_and_functions() {
    let eps_source = r#"
var x = 0;
const c = 5;
static var counter = 100;

function add_numbers(a, b) {
    var sum = a + b;
    return sum;
}

function main() {
    x += c;
    counter -= 1;
    add_numbers(x, counter);
}
"#;

    let ts = convert_eps_to_ts(eps_source).expect("conversion should succeed");
    assert!(ts.contains("import {"), "should import from @eudkit/sdk");
    assert!(ts.contains("@eudkit/sdk"));
    assert!(ts.contains("const x = var_(0);"));
    assert!(ts.contains("const c = 5;"));
    assert!(ts.contains("const counter = var_(100);"));
    assert!(ts.contains("export function add_numbers(a: Var, b: Var)"));
    assert!(ts.contains("const sum = var_(a + b);") || ts.contains("const sum = var_("));
    assert!(ts.contains("export function main()"));
}

#[test]
fn test_convert_control_flow_switch_and_once() {
    let eps_source = r#"
function test_flow(val) {
    if (val >= 10) {
        DisplayText("High");
    } else {
        DisplayText("Low");
    }

    while (val < 20) {
        val += 1;
    }

    for (var i = 0; i < 5; i++) {
        SetDeaths(Player1, Add, 1, 0);
    }

    epdswitch (val, 240) {
        case 10:
            val += 2;
            break;
        default:
            val = 0;
    }

    once {
        DisplayText("Initialized");
    }
}
"#;

    let ts = convert_eps_to_ts(eps_source).expect("conversion should succeed");
    assert!(ts.contains("if (val >= 10)"));
    assert!(ts.contains("while (val < 20)"));
    assert!(ts.contains("for (let i = 0; i < 5; i++)"));
    assert!(ts.contains("switch (val)"));
    assert!(ts.contains("case 10:"));
    assert!(ts.contains("default:"));
    assert!(ts.contains("_once_flag") || ts.contains("_once"));
    assert!(ts.contains("DisplayText(\"Initialized\")"));
}

#[test]
fn test_convert_structs_cunit_and_arrays() {
    let eps_source = r#"
var selectedUnit = 0;

function manage_unit(ptr, epd) {
    const u = CUnit(epd, ptr=ptr);
    if (u.owner == $P8) {
        u.hitPoints = 10000;
        u.posX += 10;
    }
    dwwrite_epd(epd + 0x50/4, 42);
    const arr = EUDArray(8);
    arr[0] = u.hitPoints;
}
"#;

    let ts = convert_eps_to_ts(eps_source).expect("conversion should succeed");
    assert!(ts.contains("CUnit.cast(epd, ptr)"));
    assert!(ts.contains("u.player == Player8") || ts.contains("u.owner == Player8"));
    assert!(ts.contains("u.hitPoints = 10000;"));
    assert!(ts.contains("u.posX += 10;"));
    assert!(ts.contains("SetMemory(epd + 0x50/4, SetTo, 42)") || ts.contains("dwwrite_epd("));
    assert!(ts.contains("new EUDArray(8)"));
    assert!(ts.contains("arr.set(0, u.hitPoints)") || ts.contains("arr[0] = u.hitPoints"));
}

#[test]
fn test_converted_code_compiles_to_hir_cleanly() {
    let eps_source = r#"
var x = 0;
const c = 5;

function main() {
    x += c;
    if (x >= 5) {
        DisplayText("Success");
        SetDeaths(Player1, Add, 1, 0);
    }
    PreserveTrigger();
}
"#;

    let ts = convert_eps_to_ts(eps_source).expect("conversion should succeed");
    let map_path = repo_root().join("tests/unittests/testmap.scx");
    let session = CompilationSession::from_map(&map_path, CompilationConfig::default())
        .expect("should load testmap.scx");
    let map_ctx = session.map_context();
    let hir = eudkit_ts::compile_ts_to_hir(&ts, "test_migrated", map_ctx)
        .expect("migrated TypeScript should compile to HIR cleanly");

    assert_eq!(hir.name, "test_migrated");
    assert!(hir.functions.iter().any(|f| f.name == "main"));
}

#[test]
fn cli_migrate_edd_project_and_builds_scx_cleanly() {
    let root = repo_root();
    let testmap = root.join("tests/unittests/testmap.scx");

    let temp = tempfile::tempdir().unwrap();
    let temp_path = temp.path();

    let eps_file = temp_path.join("main.eps");
    let edd_file = temp_path.join("project.edd");
    let out_scx = temp_path.join("out.scx");

    let eps_code = r#"
var counter = 10;
const minerals = 500;

function main() {
    counter += 1;
    if (counter >= 15) {
        SetResources(Player1, Add, minerals, Ore);
        DisplayText("Resources Added!");
    }
    PreserveTrigger();
}
"#;
    fs::write(&eps_file, eps_code).unwrap();

    let edd_code = format!(
        r#"[main]
input: {}
output: {}

[main.eps]
"#,
        testmap.display(),
        out_scx.display()
    );
    fs::write(&edd_file, edd_code).unwrap();

    // 1. Run `eudkit migrate project.edd`
    let migrate_out = Command::new(eudkit_bin())
        .arg("migrate")
        .arg(&edd_file)
        .current_dir(temp_path)
        .output()
        .expect("should execute eudkit migrate");

    assert!(
        migrate_out.status.success(),
        "eudkit migrate failed: {}",
        String::from_utf8_lossy(&migrate_out.stderr)
    );

    let toml_file = temp_path.join("eudkit.toml");
    let ts_file = temp_path.join("main.ts");
    assert!(toml_file.exists(), "eudkit.toml should be generated");
    assert!(ts_file.exists(), "main.ts should be generated");

    let ts_content = fs::read_to_string(&ts_file).unwrap();
    assert!(ts_content.contains("import {"));
    assert!(ts_content.contains("export function main()"));

    // 2. Run `eudkit build --config eudkit.toml` to verify full compilation to SCX
    let build_out = Command::new(eudkit_bin())
        .arg("build")
        .arg("--config")
        .arg(&toml_file)
        .current_dir(temp_path)
        .output()
        .expect("should execute eudkit build on migrated project");

    assert!(
        build_out.status.success(),
        "eudkit build failed on migrated project: {}",
        String::from_utf8_lossy(&build_out.stderr)
    );

    assert!(out_scx.exists(), "output .scx should have been generated");

    // 3. Verify output map with StormLib and Chk
    let mut archive = Archive::open(&out_scx, OpenArchiveFlags::empty()).unwrap();
    let raw_chk = archive
        .open_file("staredit\\scenario.chk")
        .unwrap()
        .read_all()
        .unwrap();
    let chk = Chk::parse(&raw_chk).unwrap();
    assert!(chk.has_isom_footer());
    assert!(chk.section(b"STRx").is_some());
    assert!(chk.section(b"TRIG").is_some());
}

#[test]
fn cli_migrate_single_eps_file() {
    let temp = tempfile::tempdir().unwrap();
    let eps_file = temp.path().join("logic.eps");
    let ts_file = temp.path().join("logic.ts");

    let eps_code = r#"
var score = 0;
function addScore(n) {
    score += n;
}
"#;
    fs::write(&eps_file, eps_code).unwrap();

    let migrate_out = Command::new(eudkit_bin())
        .arg("migrate")
        .arg(&eps_file)
        .arg("-o")
        .arg(&ts_file)
        .output()
        .expect("should execute eudkit migrate single eps");

    assert!(
        migrate_out.status.success(),
        "eudkit migrate failed: {}",
        String::from_utf8_lossy(&migrate_out.stderr)
    );

    assert!(ts_file.exists());
    let ts_content = fs::read_to_string(&ts_file).unwrap();
    assert!(ts_content.contains("const score = var_(0);"));
    assert!(ts_content.contains("export function addScore(n: Var)"));
}

#[test]
fn test_migrating_repository_eps_fixtures() {
    let root = repo_root();
    let fixtures = [
        root.join("tests/unittests/epscript/attribute.eps"),
        root.join("tests/unittests/report/C241219.eps"),
        root.join("tests/unittests/report/C240930.eps"),
    ];

    for fixture_path in &fixtures {
        assert!(fixture_path.exists(), "fixture {:?} should exist", fixture_path);
        let content = fs::read_to_string(fixture_path).unwrap();
        let ts = convert_eps_to_ts(&content).expect("fixture should convert cleanly");
        assert!(ts.contains("@eudkit/sdk"), "converted TS should import @eudkit/sdk");
    }
}

#[test]
fn cli_migrate_and_build_large_eps_fixture_to_scx() {
    let root = repo_root();
    let testmap = root.join("tests/unittests/testmap.scx");
    let c240930_eps = root.join("tests/unittests/report/C240930.eps");

    let temp = tempfile::tempdir().unwrap();
    let temp_path = temp.path();

    let out_ts = temp_path.join("c240930.ts");
    let out_scx = temp_path.join("c240930.scx");
    let toml_file = temp_path.join("eudkit.toml");

    // 1. Migrate C240930.eps -> c240930.ts
    let migrate_out = Command::new(eudkit_bin())
        .arg("migrate")
        .arg(&c240930_eps)
        .arg("-o")
        .arg(&out_ts)
        .output()
        .expect("should migrate C240930.eps");

    assert!(
        migrate_out.status.success(),
        "migrate failed: {}",
        String::from_utf8_lossy(&migrate_out.stderr)
    );
    assert!(out_ts.exists());

    // 2. Write eudkit.toml pointing to c240930.ts
    let toml_content = format!(
        r#"[project]
name = "c240930_bench"
entry = {:?}

[build]
input_map = {:?}
output_map = {:?}
obscurate = false
"#,
        out_ts.to_string_lossy(),
        testmap.to_string_lossy(),
        out_scx.to_string_lossy(),
    );
    fs::write(&toml_file, toml_content).unwrap();

    // 3. Build with eudkit
    let build_out = Command::new(eudkit_bin())
        .arg("build")
        .arg("--config")
        .arg(&toml_file)
        .current_dir(temp_path)
        .output()
        .expect("should build migrated fixture");

    assert!(
        build_out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build_out.stderr)
    );
    assert!(out_scx.exists());

    // 4. Verify output archive
    let mut archive = Archive::open(&out_scx, OpenArchiveFlags::empty()).unwrap();
    let raw_chk = archive
        .open_file("staredit\\scenario.chk")
        .unwrap()
        .read_all()
        .unwrap();
    let chk = Chk::parse(&raw_chk).unwrap();
    assert!(chk.has_isom_footer());
    assert!(chk.section(b"STRx").is_some());
    assert!(chk.section(b"TRIG").is_some());
}





