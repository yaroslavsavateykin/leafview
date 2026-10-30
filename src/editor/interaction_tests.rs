use super::*;
use helix_core::Rope;
use ratatui::layout::Rect;
use serde_json::json;
fn editor() -> Editor {
    let mut e = Editor::new();
    e.buffers.push(Buffer::from_text(
        "/tmp/a.typ".into(),
        Rope::from_str("alpha beta\nsecond\n"),
    ));
    e.buffers.push(Buffer::from_text(
        "/tmp/b.typ".into(),
        Rope::from_str("other\n"),
    ));
    e
}
#[test]
fn drag_selection_tabs_and_unicode_locations() {
    let mut e = editor();
    let area = Rect::new(0, 0, 70, 14);
    for (kind, column) in [
        (MouseEventKind::Down(MouseButton::Left), 6),
        (MouseEventKind::Drag(MouseButton::Left), 10),
    ] {
        e.mouse(
            MouseEvent {
                kind,
                column,
                row: 2,
                modifiers: KeyModifiers::NONE,
            },
            area,
        );
    }
    assert_eq!(e.vim.selection(&e.buffers[0]), Some((0, 5)));
    e.key(KeyEvent::from(KeyCode::Char('y')));
    e.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 40,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(e.current, 1);
    e.key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL));
    assert_eq!(e.current, 0);
    assert_eq!(lsp::char_position(&Rope::from_str("a😀中\n"), 0, 3), 2);
    assert_eq!(
        lsp::file_path("file:///tmp/a%20b.typ").unwrap(),
        PathBuf::from("/tmp/a b.typ")
    );
}
#[test]
fn structured_completion_and_definition() {
    let mut e = editor();
    e.buffers[0].set_cursor(5);
    e.receive_lsp(&json!([{"method":"textDocument/completion","result":{"items":[{"label":"alphabet","textEdit":{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}},"newText":"alphabet"}}]}}]).to_string());
    assert!(e.popup_text[0].contains("alphabet"));
    e.key(KeyEvent::from(KeyCode::Tab));
    e.key(KeyEvent::from(KeyCode::Enter));
    assert!(e.buffers[0]
        .state
        .doc
        .to_string()
        .starts_with("alphabet beta"));
    e.receive_lsp(&json!([{"method":"textDocument/definition","result":[{"uri":"file:///tmp/b.typ","range":{"start":{"line":0,"character":2},"end":{"line":0,"character":4}}}]}]).to_string());
    assert_eq!(e.current, 1);
    assert_eq!(e.buffers[1].cursor(), 2);
}
#[test]
fn vim_paging_marks_tabs_and_insert_controls() {
    let mut e = editor();
    e.buffers[0] = Buffer::from_text(
        "/tmp/a.typ".into(),
        Rope::from_str(&"alpha beta\n".repeat(100)),
    );
    e.viewport_height = 10;
    e.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(e.buffers[0].line(), 5);
    for c in ['m', 'a', 'g', 'g', '`', 'a'] {
        e.key(KeyEvent::from(KeyCode::Char(c)));
    }
    assert_eq!(e.buffers[0].line(), 5);
    for c in ['g', 't'] {
        e.key(KeyEvent::from(KeyCode::Char(c)));
    }
    assert_eq!(e.current, 1);
    for c in ['g', 'T'] {
        e.key(KeyEvent::from(KeyCode::Char(c)));
    }
    assert_eq!(e.current, 0);
    e.key(KeyEvent::from(KeyCode::Char('A')));
    e.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    assert!(e.buffers[0]
        .state
        .doc
        .line(5)
        .to_string()
        .starts_with("alpha "));
    e.key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(e.buffers[0].state.doc.line(5).to_string(), "\n");
    e.key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL));
    assert_eq!(e.vim.mode, vim::Mode::Normal);
}
#[test]
fn autocomplete_debounce_and_cancel() {
    let mut e = editor();
    e.visible = true;
    e.focused = true;
    e.key(KeyEvent::from(KeyCode::Char('i')));
    e.key(KeyEvent::from(KeyCode::Char('a')));
    assert_eq!(e.auto_due.as_ref().unwrap().1, "complete");
    assert!(e.auto_due.as_ref().unwrap().0 > std::time::Instant::now());
    e.key(KeyEvent::from(KeyCode::Char('(')));
    assert!(e.signature_due.is_some());
    assert!(e.auto_due.is_none());
    e.key(KeyEvent::from(KeyCode::Esc));
    assert!(e.auto_due.is_none());
    assert!(e.lsp_pending.is_none());
}
#[test]
#[ignore = "requires external tinymist on PATH"]
fn real_automatic_completion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.typ");
    std::fs::write(&path, "#let greeting = 1\n#gree").unwrap();
    let mut e = Editor::new();
    e.buffers.push(Buffer::from_text(
        path,
        Rope::from_str("#let greeting = 1\n#gree"),
    ));
    e.visible = true;
    e.focused = true;
    let end = e.buffers[0].state.doc.len_chars();
    e.buffers[0].set_cursor(end);
    e.vim.mode = vim::Mode::Insert;
    e.key(KeyEvent::from(KeyCode::Char('t')));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while e.popup.is_none() {
        e.tick();
        e.poll();
        assert!(std::time::Instant::now() < deadline, "{}", e.message);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        e.popup_text.iter().any(|s| s.contains("greeting")),
        "{:?}",
        e.popup_text
    );
    let (_, items, selected) = e.popup.as_mut().unwrap();
    *selected = items
        .iter()
        .position(|v| v["label"].as_str().is_some_and(|s| s.contains("greeting")))
        .unwrap();
    let chosen = items[*selected].clone();
    e.cmp.selected = true;
    e.key(KeyEvent::from(KeyCode::Enter));
    assert!(
        e.buffers[0].state.doc.to_string().ends_with("#greeting"),
        "doc={} message={} item={chosen}",
        e.buffers[0].state.doc,
        e.message
    );
}
#[test]
#[ignore = "requires external tinymist and typstyle on PATH"]
fn real_typst_lsp_and_formatter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.typ");
    let text = Rope::from_str("#let greet(name) = [Hello #name]\n#greet(\"world\")\n");
    std::fs::write(&path, text.to_string()).unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut runner = tasks::Runner::default();
    for command in ["hover", "definition", "signature", "complete"] {
        let cursor = if command == "signature" { 39 } else { 34 };
        let reply = runner
            .run(
                Task::Lsp(path.clone(), 1, text.clone(), cursor, command.into()),
                &stop,
            )
            .unwrap();
        let Reply::Lsp(_, _, reply) = reply else {
            panic!("not an LSP response")
        };
        let messages: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert!(messages[0]["error"].is_null(), "{command}: {reply}");
        assert!(!messages[0]["result"].is_null(), "{command}: {reply}");
    }
    let Reply::Formatted(_, _, result) = runner
        .run(
            Task::Format(path, 1, Rope::from_str("#let x=(1,2,3)\n")),
            &stop,
        )
        .unwrap()
    else {
        panic!("not formatted")
    };
    assert!(result.contains("(1, 2, 3)"), "{result}");
}
