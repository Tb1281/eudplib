use std::process::Command;

#[test]
fn version_flag_prints_binary_name_and_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
        .arg("--version")
        .output()
        .expect("eudkit-lsp --version should run");

    assert!(
        output.status.success(),
        "eudkit-lsp --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("eudkit-lsp {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_arguments_fail_with_usage_instead_of_starting_the_server() {
    let output = Command::new(env!("CARGO_BIN_EXE_eudkit-lsp"))
        .arg("--definitely-not-an-option")
        .output()
        .expect("eudkit-lsp should reject unknown arguments");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("usage: eudkit-lsp"), "{stderr}");
}
