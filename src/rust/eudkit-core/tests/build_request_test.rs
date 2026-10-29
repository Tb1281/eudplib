use eudkit_core::config::{BuildRequest, PROJECT_FILE};
use std::fs;
use std::path::PathBuf;

fn parse(args: &[&str]) -> BuildRequest {
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    BuildRequest::from_args(&args).expect("should parse build request")
}

#[test]
fn from_args_parses_the_shared_build_flags() {
    let request = parse(&[
        "--config",
        "custom.toml",
        "--input",
        "in.scx",
        "--output",
        "out.scx",
        "--entry",
        "main.py",
        "--dts",
        "map.d.ts",
        "--obscurate",
        "--seed",
        "77",
        "--watch",
        "--once",
        "--hir-only",
        "--lir-only",
    ]);

    assert_eq!(request.config, Some(PathBuf::from("custom.toml")));
    assert_eq!(request.input_map, Some(PathBuf::from("in.scx")));
    assert_eq!(request.output_map, Some(PathBuf::from("out.scx")));
    assert_eq!(request.entry, Some(PathBuf::from("main.py")));
    assert_eq!(request.dts, Some(PathBuf::from("map.d.ts")));
    assert_eq!(request.obscurate, Some(true));
    assert_eq!(request.obscurate_seed, Some(77));
    assert!(request.watch);
    assert!(request.once);
    assert!(request.hir_only);
    assert!(request.lir_only);
    assert!(!request.help);
}

#[test]
fn from_args_accepts_map_alias_and_one_positional_input() {
    assert_eq!(
        parse(&["--map", "a.scx"]).input_map,
        Some(PathBuf::from("a.scx"))
    );
    assert_eq!(
        parse(&["-m", "b.scx"]).input_map,
        Some(PathBuf::from("b.scx"))
    );
    assert_eq!(parse(&["c.scx"]).input_map, Some(PathBuf::from("c.scx")));

    let args = ["a.scx".to_string(), "b.scx".to_string()];
    assert!(BuildRequest::from_args(&args).is_err());
}

#[test]
fn from_args_rejects_unknown_flags_and_missing_values() {
    let unknown = ["--nope".to_string()];
    let error = BuildRequest::from_args(&unknown).unwrap_err().to_string();
    assert!(
        error.contains("unknown argument for build: --nope"),
        "{error}"
    );

    let missing = ["--input".to_string()];
    let error = BuildRequest::from_args(&missing).unwrap_err().to_string();
    assert!(error.contains("missing value for --input"), "{error}");

    let bad_seed = ["--seed".to_string(), "fast".to_string()];
    let error = BuildRequest::from_args(&bad_seed).unwrap_err().to_string();
    assert!(error.contains("invalid --seed value: fast"), "{error}");
}

#[test]
fn resolve_applies_flags_over_both_config_sections() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(PROJECT_FILE),
        r#"
name = "top_level"
entry = "entry.py"
input_map = "top_in.scx"
output_map = "top_out.scx"
obscurate = false
obscurate_seed = 11
preserve_classic_triggers = false

[project]
input_map = "project_in.scx"

[build]
input_map = "build_in.scx"
output_map = "build_out.scx"
obscurate_seed = 22
"#,
    )
    .unwrap();

    let request = BuildRequest {
        input_map: Some(PathBuf::from("flag_in.scx")),
        obscurate: Some(true),
        obscurate_seed: Some(33),
        ..BuildRequest::default()
    };
    let resolved = request.resolve_in(temp.path()).unwrap();

    assert_eq!(resolved.config_path, Some(temp.path().join(PROJECT_FILE)));
    assert_eq!(resolved.config.name, "top_level");
    assert_eq!(resolved.config.input_map, PathBuf::from("flag_in.scx"));
    assert_eq!(
        resolved.config.output_map,
        temp.path().join("build_out.scx")
    );
    assert_eq!(resolved.config.entry, Some(PathBuf::from("entry.py")));
    assert!(resolved.config.obscurate);
    assert_eq!(resolved.config.obscurate_seed, Some(33));
    assert!(!resolved.config.preserve_classic_triggers);
}

#[test]
fn resolve_discovers_the_project_file_and_explicit_config_wins() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join(PROJECT_FILE),
        "[project]\nname = \"discovered\"\ninput_map = \"map.scx\"\n",
    )
    .unwrap();

    let discovered = BuildRequest::default().resolve_in(temp.path()).unwrap();
    assert_eq!(discovered.config_path, Some(temp.path().join(PROJECT_FILE)));
    assert_eq!(discovered.config.name, "discovered");
    assert_eq!(discovered.config.input_map, PathBuf::from("map.scx"));

    let explicit_path = temp.path().join("custom.toml");
    fs::write(
        &explicit_path,
        "[project]\nname = \"explicit\"\n\n[build]\noutput_map = \"out.scx\"\n",
    )
    .unwrap();
    let explicit = BuildRequest {
        config: Some(PathBuf::from("custom.toml")),
        ..BuildRequest::default()
    }
    .resolve_in(temp.path())
    .unwrap();
    assert_eq!(explicit.config_path, Some(explicit_path));
    assert_eq!(explicit.config.name, "explicit");
}

#[test]
fn resolve_without_a_project_file_returns_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let resolved = BuildRequest::default().resolve_in(temp.path()).unwrap();

    assert_eq!(resolved.config_path, None);
    assert_eq!(resolved.config.name, "eudkit_project");
    assert!(resolved.config.input_map.as_os_str().is_empty());
    assert!(resolved.config.output_map.as_os_str().is_empty());
    assert!(resolved.config.preserve_classic_triggers);
    assert!(!resolved.config.obscurate);
}
