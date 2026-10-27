use eudkit_core::chk::Chk;
use eudkit_core::config::ProjectConfig;
use eudplib_stormlib::{Archive, OpenArchiveFlags};
use std::fs;
use std::path::PathBuf;
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
fn cli_inspect_dump_ir_and_build_from_eudkit_toml_work_end_to_end() {
    let root = repo_root();
    let testmap = root.join("tests/unittests/testmap.scx");

    // 1. Parse eudkit.toml config
    let toml_text = r#"
[project]
name = "tracer_demo"
entry = "src/main.ts"

[build]
input_map = "tests/unittests/testmap.scx"
output_map = "build/out.scx"
obscurate = false
obscurate_seed = 42
"#;
    let cfg = ProjectConfig::parse_str(toml_text).expect("should parse eudkit.toml");
    assert_eq!(cfg.name, "tracer_demo");
    assert_eq!(cfg.input_map, PathBuf::from("tests/unittests/testmap.scx"));
    assert_eq!(cfg.output_map, PathBuf::from("build/out.scx"));
    assert!(!cfg.obscurate);
    assert_eq!(cfg.obscurate_seed, Some(42));

    // 2. Run `eudkit inspect <map.scx> --dts <types.d.ts>`
    let temp = tempfile::tempdir().unwrap();
    let inspect_dts_path = temp.path().join("inspected_map.d.ts");
    let inspect_out = Command::new(eudkit_bin())
        .arg("inspect")
        .arg(&testmap)
        .arg("--dts")
        .arg(&inspect_dts_path)
        .output()
        .expect("should execute eudkit inspect");
    assert!(
        inspect_out.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&inspect_out.stderr)
    );
    let inspect_stdout = String::from_utf8_lossy(&inspect_out.stdout);
    assert!(inspect_stdout.contains("Strings:"));
    assert!(inspect_stdout.contains("Locations:"));
    assert!(inspect_stdout.contains("Placed Units:"));
    assert!(inspect_stdout.contains("CUWP Slots:"));
    assert!(inspect_stdout.contains("Switches:"));
    assert!(inspect_stdout.contains("Players:"));
    assert!(inspect_dts_path.exists(), "inspect --dts should generate .d.ts");
    let dts_content = fs::read_to_string(&inspect_dts_path).unwrap();
    assert!(dts_content.contains("export type LocationId ="));
    assert!(dts_content.contains("export type UnitId ="));
    assert!(dts_content.contains("export type SwitchId ="));

    // 3. Run `eudkit dump-ir --map <map.scx>`
    let dump_out = Command::new(eudkit_bin())
        .arg("dump-ir")
        .arg("--map")
        .arg(&testmap)
        .output()
        .expect("should execute eudkit dump-ir");
    assert!(
        dump_out.status.success(),
        "dump-ir failed: {}",
        String::from_utf8_lossy(&dump_out.stderr)
    );
    let dump_stdout = String::from_utf8_lossy(&dump_out.stdout);
    assert!(dump_stdout.contains("module @"));
    assert!(dump_stdout.contains("lir_module @"));
    assert!(dump_stdout.contains("NextPtr"));

    // 4. Run `eudkit build --config <eudkit.toml>` twice and verify determinism + StormLib validity
    let temp = tempfile::tempdir().unwrap();
    let out1 = temp.path().join("cli_out1.scx");
    let out2 = temp.path().join("cli_out2.scx");
    let config_path = temp.path().join("eudkit.toml");
    fs::write(
        &config_path,
        format!(
            "[project]\nname = \"cli_test\"\n\n[build]\ninput_map = {:?}\noutput_map = {:?}\nobscurate = false\n",
            testmap.to_string_lossy(),
            out1.to_string_lossy()
        ),
    )
    .unwrap();

    let build1 = Command::new(eudkit_bin())
        .arg("build")
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("should execute eudkit build 1");
    assert!(
        build1.status.success(),
        "build 1 failed: {}",
        String::from_utf8_lossy(&build1.stderr)
    );

    let build2 = Command::new(eudkit_bin())
        .arg("build")
        .arg("--input")
        .arg(&testmap)
        .arg("--output")
        .arg(&out2)
        .output()
        .expect("should execute eudkit build 2");
    assert!(
        build2.status.success(),
        "build 2 failed: {}",
        String::from_utf8_lossy(&build2.stderr)
    );

    let b1 = fs::read(&out1).unwrap();
    let b2 = fs::read(&out2).unwrap();
    assert_eq!(b1, b2, "CLI build output must be byte-for-byte deterministic");

    let mut archive = Archive::open(&out1, OpenArchiveFlags::empty()).unwrap();
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
fn cli_build_with_typescript_entry_produces_deterministic_scx() {
    let root = repo_root();
    let testmap = root.join("tests/unittests/testmap.scx");

    let temp = tempfile::tempdir().unwrap();
    let ts_file = temp.path().join("main.ts");
    let dts_file = temp.path().join("types/map.d.ts");
    let out1 = temp.path().join("ts_out1.scx");
    let out2 = temp.path().join("ts_out2.scx");
    let config_path = temp.path().join("eudkit.toml");

    let ts_code = r#"
import { var_, SetDeaths, SetResources, PreserveTrigger, Player1, Ore, Add } from "@eudkit/sdk";

const counter = var_(10);
const minerals = var_(500);

export function main() {
    counter.add(1);
    if (counter.atLeast(15)) {
        minerals.add(100);
        SetResources(Player1, Add, 100, Ore);
    }
    for (let i = 0; i < 3; i++) {
        SetDeaths(Player1, Add, 5, 0);
    }
    PreserveTrigger();
}
"#;
    fs::write(&ts_file, ts_code).unwrap();

    let toml_code = format!(
        r#"
[project]
name = "ts_end_to_end"
entry = {:?}
dts = {:?}

[build]
input_map = {:?}
output_map = {:?}
obscurate = false
"#,
        ts_file.to_string_lossy(),
        dts_file.to_string_lossy(),
        testmap.to_string_lossy(),
        out1.to_string_lossy(),
    );
    fs::write(&config_path, toml_code).unwrap();

    // 1. Run dump-ir with config pointing to TS entry
    let dump_out = Command::new(eudkit_bin())
        .arg("dump-ir")
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("should execute eudkit dump-ir with TS entry");
    assert!(
        dump_out.status.success(),
        "dump-ir with TS failed: {}",
        String::from_utf8_lossy(&dump_out.stderr)
    );
    let dump_stdout = String::from_utf8_lossy(&dump_out.stdout);
    assert!(dump_stdout.contains("module @ts_end_to_end"));
    assert!(dump_stdout.contains("fn @main"));
    assert!(dump_stdout.contains("counter"));

    // 2. Run build 1
    let build1 = Command::new(eudkit_bin())
        .arg("build")
        .arg("--config")
        .arg(&config_path)
        .output()
        .expect("should execute eudkit build 1 with TS");
    assert!(
        build1.status.success(),
        "build 1 failed: {}",
        String::from_utf8_lossy(&build1.stderr)
    );
    let build1_stdout = String::from_utf8_lossy(&build1.stdout);
    assert!(build1_stdout.contains("Built"));
    assert!(build1_stdout.contains("triggers="));

    // Verify .d.ts was generated
    assert!(dts_file.exists(), ".d.ts file should have been generated during build");
    let dts_content = fs::read_to_string(&dts_file).unwrap();
    assert!(dts_content.contains("export type UnitId ="));
    assert!(dts_content.contains("export interface MapContext"));

    // 3. Run build 2 to verify byte-for-byte determinism
    let build2 = Command::new(eudkit_bin())
        .arg("build")
        .arg("--config")
        .arg(&config_path)
        .arg("--output")
        .arg(&out2)
        .output()
        .expect("should execute eudkit build 2 with TS");
    assert!(
        build2.status.success(),
        "build 2 failed: {}",
        String::from_utf8_lossy(&build2.stderr)
    );

    let b1 = fs::read(&out1).unwrap();
    let b2 = fs::read(&out2).unwrap();
    assert_eq!(b1, b2, "TS build output must be byte-for-byte deterministic");

    // 4. Validate output archive with StormLib and Chk
    let mut archive = Archive::open(&out1, OpenArchiveFlags::empty()).unwrap();
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
