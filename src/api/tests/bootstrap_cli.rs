use std::{
    io::Write,
    process::{Command, Stdio},
};
#[test]
fn bootstrap_from_stdin_does_not_open_a_listener_or_print_the_password() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_keyword-vault"))
        .args(["bootstrap", "local-admin"])
        .env("KV_DATA_DIR", dir.path())
        .env("KV_BIND", "invalid-bind-must-not-be-used")
        .env_remove("ACCESS_PUBLIC_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"stdin-password-canary-123\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "bootstrap command should initialize without binding TCP"
    );
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("created"));
    assert!(!printed.contains("stdin-password-canary"));
}
