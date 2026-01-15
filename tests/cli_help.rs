use std::process::Command;

#[test]
fn help_includes_cli_flags() {
    let exe = env!("CARGO_BIN_EXE_greentic-demo");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("failed to run greentic-demo --help");

    assert!(output.status.success(), "help command failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    let flags = [
        "--packs-dir",
        "--port",
        "--secrets-backend",
        "--pack-source",
        "--pack-index-url",
        "--pack-cache-dir",
        "--pack-public-key",
        "--pack-refresh-interval",
        "--pack-refresh-interval-secs",
        "--tenant-resolver",
    ];

    for flag in flags {
        assert!(
            combined.contains(flag),
            "expected help output to mention {flag}"
        );
    }
}
