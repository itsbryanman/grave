use std::process::Command;

#[test]
fn help_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("--help")
        .output()
        .expect("run grave --help");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    assert!(output.stderr.is_empty());
}

#[test]
fn subcommand_help_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("help")
        .arg("exhume")
        .output()
        .expect("run grave help exhume");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("grave exhume"));
    assert!(output.stderr.is_empty());
}

#[test]
fn version_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("--version")
        .output()
        .expect("run grave --version");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
    assert!(output.stderr.is_empty());
}
