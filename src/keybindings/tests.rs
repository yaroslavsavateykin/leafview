use super::*;
use engine::{Binding, Engine, Resolution};
use registry::Invocation;
use std::time::{Duration, Instant};
fn binding(key: &str, command: &str, priority: i32) -> Binding {
    Binding {
        context: "Editor".into(),
        mode: "normal".into(),
        keys: keys::sequence(key, "Space", ",").unwrap(),
        invocation: Some(Invocation::new(command)),
        priority,
        source: "test".into(),
    }
}
fn feed(e: &mut Engine, key: &str) -> Resolution {
    e.feed(
        keys::Stroke::parse(key).unwrap(),
        &["Editor", "Global"],
        "normal",
        Instant::now(),
    )
}
#[test]
fn prefixes_and_timeout() {
    let now = Instant::now();
    let mut e = Engine::compile(
        vec![
            binding("g", "g", 0),
            binding("gg", "gg", 0),
            binding("go", "go", 0),
            binding("goo", "goo", 0),
        ],
        50,
    );
    assert!(matches!(feed(&mut e, "g"), Resolution::Pending));
    assert!(matches!(feed(&mut e,"g"),Resolution::Execute(v) if v.command=="gg"));
    feed(&mut e, "g");
    feed(&mut e, "o");
    assert!(
        matches!(e.expire(now+Duration::from_secs(1)),Some(Resolution::Execute(v)) if v.command=="go")
    );
    feed(&mut e, "g");
    feed(&mut e, "o");
    assert!(matches!(feed(&mut e,"o"),Resolution::Execute(v) if v.command=="goo"));
}
#[test]
fn leader_escape_count_and_conflict() {
    let mut e = Engine::compile(
        vec![
            binding("<Leader>o", "a", 0),
            binding("<Leader>oa", "b", 0),
            binding("<Leader>on", "c", 0),
            binding("j", "old", 0),
            binding("j", "new", 1000),
        ],
        100,
    );
    feed(&mut e, "Space");
    assert_eq!(e.hints()[0].0, "o");
    assert!(matches!(feed(&mut e, "Esc"), Resolution::Consumed));
    feed(&mut e, "5");
    assert!(matches!(feed(&mut e,"j"),Resolution::Execute(v) if v.command=="new"&&v.count==5));
    assert!(e.conflicts.iter().any(|c| !c.prefix));
}
#[test]
fn override_disable_and_partial_config() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("keys.toml");
    std::fs::write(&p,"[keybindings]\npreset='astronvim'\n[keybindings.bindings.normal]\nj='editor.cursor.up'\nk={disabled=true}\n").unwrap();
    let r = registry::Registry::builtins();
    let (c, b) = config::load(&[p], &r).unwrap();
    let mut e = Engine::compile(b, c.timeout_ms);
    assert!(matches!(feed(&mut e,"j"),Resolution::Execute(v) if v.command=="editor.cursor.up"));
    assert!(matches!(feed(&mut e, "k"), Resolution::Consumed));
    assert!(matches!(feed(&mut e, "Space"), Resolution::Pending));
    assert!(ConfigError::invalid());
}
#[test]
fn ui_record_export_and_conflict_render() {
    let dir = tempfile::tempdir().unwrap();
    let mut system = System::new(dir.path());
    system.paths = vec![dir.path().join("keybindings.toml")];
    system.reload().unwrap();
    system.open(true);
    use crossterm::event::{KeyCode, KeyEvent};
    for c in "editor.save".chars() {
        system.panel_key(KeyEvent::from(KeyCode::Char(c)));
    }
    system.panel_key(KeyEvent::from(KeyCode::Enter));
    system.panel_key(KeyEvent::from(KeyCode::Char('s')));
    system.panel_key(KeyEvent::from(KeyCode::Enter));
    let saved = std::fs::read_to_string(&system.paths[0]).unwrap();
    assert!(saved.contains("editor.save"));
    assert!(
        matches!(feed(&mut system.engine,"s"),Resolution::Execute(v) if v.command=="editor.save")
    );
    system.panel_key(KeyEvent::from(KeyCode::F(7)));
    assert!(dir.path().join("keybindings-export.toml").exists());
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 25)).unwrap();
    terminal.draw(|f| system.render(f)).unwrap();
    let rows = terminal
        .backend()
        .buffer()
        .content
        .chunks(120)
        .map(|r| r.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    assert!(rows.join("\n").contains("Keyboard Shortcuts"));
    if let Some(path) = std::env::var_os("LEAFVIEW_KEYS_RENDER") {
        std::fs::write(path, serde_json::to_string(&rows).unwrap()).unwrap();
    }
}
#[test]
fn layer_scalar_inheritance_localleader_args_and_temporary() {
    let dir = tempfile::tempdir().unwrap();
    let global = dir.path().join("a.toml");
    let project = dir.path().join("b.toml");
    std::fs::write(&global,"[keybindings]\nleader='x'\ntimeout_ms=75\n[keybindings.normal]\n'<LocalLeader>h'={command='editor.cursor.left',args={test=1}}\n").unwrap();
    std::fs::write(&project, "[keybindings.normal]\nz='editor.cursor.right'\n").unwrap();
    let registry = registry::Registry::builtins();
    let (c, b) = config::load(&[global, project], &registry).unwrap();
    assert_eq!(c.leader, "x");
    assert_eq!(c.timeout_ms, 75);
    let mut engine = Engine::compile(b, c.timeout_ms);
    feed(&mut engine, ",");
    assert!(matches!(feed(&mut engine,"h"),Resolution::Execute(v) if v.args["test"]==1));
    let mut system = System::new(dir.path());
    system
        .override_temporarily(vec![binding("j", "editor.cursor.up", 0)])
        .unwrap();
    assert!(
        matches!(feed(&mut system.engine,"j"),Resolution::Execute(v) if v.command=="editor.cursor.up")
    );
}
#[test]
fn examples_load_and_round_trip() {
    for input in [
        include_str!("../../docs/keybindings/examples/astronvim.toml"),
        include_str!("../../docs/keybindings/examples/vim.toml"),
        include_str!("../../docs/keybindings/examples/default.toml"),
        include_str!("../../docs/keybindings/examples/custom.toml"),
    ] {
        let c = config::Config::parse(input).unwrap();
        let text = c.export().unwrap();
        assert_eq!(config::Config::parse(&text).unwrap().preset, c.preset);
    }
}
struct ConfigError;
impl ConfigError {
    fn invalid() -> bool {
        config::Config::parse("[keybindings]\nunknown=2").is_err()
    }
}
#[test]
fn context_and_text_input() {
    let mut b = binding("j", "move", 0);
    b.context = "PDF".into();
    let mut e = Engine::compile(vec![b], 100);
    assert!(matches!(
        e.feed(
            keys::Stroke::parse("j").unwrap(),
            &["Search"],
            "insert",
            Instant::now()
        ),
        Resolution::Pass
    ));
    assert!(matches!(
        e.feed(
            keys::Stroke::parse("j").unwrap(),
            &["PDF"],
            "normal",
            Instant::now()
        ),
        Resolution::Execute(_)
    ));
}
#[test]
fn unicode_and_modifiers() {
    let e = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('J'),
        crossterm::event::KeyModifiers::SHIFT,
    );
    assert_eq!(keys::Stroke::event(e), keys::Stroke::parse("J").unwrap());
    assert_eq!(keys::sequence("ёж", "Space", ",").unwrap().len(), 2);
    assert!(keys::Stroke::parse("Ctrl+Alt+Super+x")
        .unwrap()
        .to_event()
        .is_some());
}
#[test]
#[ignore = "resolution microbenchmark"]
fn benchmark_ten_thousand_bindings() {
    let bindings = (0..10000)
        .map(|i| binding(&format!("Space x{i}"), "command", 0))
        .chain([binding("j", "down", 0)])
        .collect();
    let mut e = Engine::compile(bindings, 500);
    let key = keys::Stroke::parse("j").unwrap();
    let start = Instant::now();
    for _ in 0..100000 {
        let _ = e.feed(key.clone(), &["Editor"], "normal", Instant::now());
    }
    eprintln!("100k lookups: {:?}", start.elapsed());
    assert!(start.elapsed() < Duration::from_secs(5));
}
