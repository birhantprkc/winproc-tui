use std::process::{Command, Output};

fn run_binary(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_winproc-tui"))
        .args(arguments)
        .output()
        .expect("winproc-tui binary should start")
}

#[test]
fn version_reports_the_cargo_package_version() {
    let output = run_binary(&["--version"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::str::from_utf8(&output.stdout).unwrap(),
        format!("winproc-tui {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn help_describes_the_binary_entrypoint() {
    let output = run_binary(&["--help"]);

    assert!(output.status.success(), "{output:?}");
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert!(
        stdout.contains("Windows process investigation TUI"),
        "{stdout}"
    );
    assert!(stdout.contains("Usage: winproc-tui"), "{stdout}");
    assert!(stdout.contains("--help"), "{stdout}");
    assert!(stdout.contains("--version"), "{stdout}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn unknown_option_is_rejected_before_terminal_startup() {
    let output = run_binary(&["--removed-option"]);

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = std::str::from_utf8(&output.stderr).unwrap();
    assert!(stderr.contains("--removed-option"), "{stderr}");
    assert!(stderr.contains("--help"), "{stderr}");
}
