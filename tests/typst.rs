//! End-to-end compiler process tests. No installed Typst CLI needed.
#![cfg(feature = "typst")]
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
struct Helper {
    child: std::process::Child,
    input: std::process::ChildStdin,
    dir: tempfile::TempDir,
}
impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Helper {
    fn new() -> Self {
        Self::with_options(serde_json::json!({}))
    }
    fn with_options(options: Value) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("chapters")).unwrap();
        let main = dir.path().join("main.typ");
        std::fs::write(&main, include_str!("fixtures/typst-live/main.typ")).unwrap();
        std::fs::write(
            dir.path().join("chapters/theory.typ"),
            include_str!("fixtures/typst-live/chapters/theory.typ"),
        )
        .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_leafview"))
            .arg("--typst-compiler")
            .arg(&main)
            .arg(dir.path())
            .arg(serde_json::to_string(&options).unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        Self { child, input, dir }
    }
    fn compile(&mut self, generation: u64) -> Value {
        let pdf = self.dir.path().join(format!("{generation}.pdf"));
        writeln!(
            self.input,
            "{}",
            json!({"generation":generation,"output":pdf})
        )
        .unwrap();
        let reply = pdf.with_extension("json");
        let deadline = Instant::now() + Duration::from_secs(30);
        while !reply.exists() {
            assert!(Instant::now() < deadline);
            assert!(self.child.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(10));
        }
        serde_json::from_slice(&std::fs::read(reply).unwrap()).unwrap()
    }
}
#[test]
fn same_generation_pdf_mapping_includes_formula_shift_error_recovery() {
    let mut helper = Helper::new();
    let result = helper.compile(1);
    assert!(result["error"].is_null(), "{result}");
    let points = result["points"].as_array().unwrap();
    let theory = helper.dir.path().join("chapters/theory.typ");
    let relevant: Vec<_> = points
        .iter()
        .filter(|p| p["file"].as_str() == theory.to_str())
        .collect();
    assert!(!relevant.is_empty());
    assert!(relevant.iter().all(|p| p["page"] == 6));
    let text = std::fs::read_to_string(&theory).unwrap();
    let offset = text.find('E').unwrap();
    let formula = relevant
        .iter()
        .find(|p| {
            p["start"].as_u64().unwrap() <= offset as u64
                && p["end"].as_u64().unwrap() > offset as u64
        })
        .expect("formula has a glyph mapping");
    assert!(formula["y"].as_f64().unwrap() > 0.3);
    assert!(formula["y"].as_f64().unwrap() < 0.9);
    let main = helper.dir.path().join("main.typ");
    let source = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("Extra page\n#pagebreak()\n{source}")).unwrap();
    let shifted = helper.compile(2);
    assert!(shifted["error"].is_null());
    let shifted_points: Vec<_> = shifted["points"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["file"].as_str() == theory.to_str())
        .collect();
    assert!(shifted_points.iter().all(|p| p["page"] == 7));
    let bytes = std::fs::read(helper.dir.path().join("2.pdf")).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    std::fs::write(&theory, "#let broken = (\n").unwrap();
    let broken = helper.compile(3);
    assert!(broken["error"].is_string());
    assert!(!helper.dir.path().join("3.pdf").exists());
    std::fs::write(&theory, text.replace("m c^2", "m c^3")).unwrap();
    let fixed = helper.compile(4);
    assert!(fixed["error"].is_null());
    assert_eq!(fixed["generation"], 4);
}

#[test]
fn compiler_options_inputs_pages_and_pdf_mapping() {
    let mut helper = Helper::with_options(
        json!({"inputs":[["message","FROM_INPUT"]],"pages":"7-8", "pretty":true,"no_tags":true, "standards":["1.7"],"ignore_system":true,"jobs":1,"timestamp":1700000000}),
    );
    let theory = helper.dir.path().join("chapters/theory.typ");
    std::fs::write(&theory, "#sys.inputs.message\n\n$ E = m c^2 $\n").unwrap();
    let reply = helper.compile(1);
    assert!(reply["error"].is_null(), "{reply}");
    assert!(reply["points"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["file"].as_str() == theory.to_str())
        .all(|p| p["page"] == 0));
    let pdf = lopdf::Document::load(helper.dir.path().join("1.pdf")).unwrap();
    assert_eq!(pdf.get_pages().len(), 2);
    assert!(pdf.extract_text(&[1]).unwrap().contains("FROM_INPUT"));
}
