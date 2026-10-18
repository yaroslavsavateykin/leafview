use std::process::Command;

#[test]
fn prints_usage_without_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .output()
        .expect("start leafview");
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Usage: leafview"));
}

#[test]
fn missing_document_fails_without_switching_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .arg("/this/path/does/not/exist.pdf")
        .output()
        .expect("start leafview");
    assert!(!output.status.success());
}
