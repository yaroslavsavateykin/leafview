use std::process::Command;

#[test]
fn extracts_snapshot_links_in_bounded_helper() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .arg("--extract-links")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/links.pdf"
        ))
        .output()
        .unwrap();
    assert!(output.status.success());
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["pages"][0]["links"].as_array().unwrap().len(), 8);
    assert_eq!(
        data["pages"][0]["links"][1]["action"]["Internal"]["page"],
        1
    );
}

#[test]
fn prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .arg("--help")
        .output()
        .expect("start leafview");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: leafview"));
}

#[cfg(not(feature = "typst"))]
#[test]
fn typst_without_feature_explains_installation() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .args([
            "typst",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/typst-live/main.typ"
            ),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--features typst"));
    assert!(!output.stdout.windows(8).any(|w| w == b"\x1b[?1049h"));
}

#[cfg(not(feature = "typst"))]
#[test]
fn implicit_typst_without_feature_explains_installation() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/typst-live/main.typ"
        ))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--features typst"));
    assert!(!output.stdout.windows(8).any(|w| w == b"\x1b[?1049h"));
}

#[test]
fn missing_document_fails_without_switching_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_leafview"))
        .arg("/this/path/does/not/exist.pdf")
        .output()
        .expect("start leafview");
    assert!(!output.status.success());
}
