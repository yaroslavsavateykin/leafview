use super::*;
use helix_core::Rope;
use serde_json::json;
fn editor() -> Editor {
    let mut e = Editor::new();
    e.completion_config = completion::Config::default();
    e.buffers.push(Buffer::from_text(
        "/tmp/main.rs".into(),
        Rope::from_str("pri"),
    ));
    e.buffers[0].set_cursor(3);
    e.vim.mode = vim::Mode::Insert;
    e.visible = true;
    e.focused = true;
    e
}
fn show(e: &mut Editor, items: serde_json::Value) {
    e.receive_lsp(
        &json!([{"method":"textDocument/completion","result":{"items":items}}]).to_string(),
    );
}
#[test]
fn blink_no_preselect_preview_restore_and_accept() {
    let mut e = editor();
    e.completion_config.auto_insert = true;
    show(&mut e, json!([{"label":"print"},{"label":"println"}]));
    assert!(!e.cmp.selected);
    e.key(KeyEvent::from(KeyCode::Tab));
    assert_eq!(e.buffers[0].state.doc.to_string(), "print");
    e.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert_eq!(e.buffers[0].state.doc.to_string(), "println");
    e.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri");
    assert!(e.popup.is_none());
    show(&mut e, json!([{"label":"print"}]));
    e.key(KeyEvent::from(KeyCode::Tab));
    e.key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(e.buffers[0].state.doc.to_string(), "print");
    e.buffers[0].undo(false);
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri");
}
#[test]
fn snippet_additional_edits_unicode_and_tabstops() {
    let mut e = editor();
    e.buffers[0] = Buffer::from_text("/tmp/main.rs".into(), Rope::from_str("// 😀\npri"));
    e.buffers[0].set_cursor(8);
    show(
        &mut e,
        json!([{"label":"print","insertTextFormat":2,"textEdit":{"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}},"newText":"print(${1:value}, ${2|a,b|})$0"},"additionalTextEdits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"use lib;\n"}]}]),
    );
    e.key(KeyEvent::from(KeyCode::Tab));
    e.key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(
        e.buffers[0].state.doc.to_string(),
        "use lib;\n// 😀\nprint(value, a)"
    );
    e.key(KeyEvent::from(KeyCode::Char('x')));
    e.auto_due = None;
    assert!(e.buffers[0].state.doc.to_string().ends_with("print(x, a)"));
    e.popup = None;
    e.key(KeyEvent::from(KeyCode::Tab));
    e.key(KeyEvent::from(KeyCode::Char('y')));
    assert!(e.buffers[0].state.doc.to_string().ends_with("print(x, y)"));
}
#[test]
fn defaults_resolve_and_render() {
    let mut e = editor();
    e.receive_lsp(&json!([{"method":"textDocument/completion","result":{"itemDefaults":{"insertTextFormat":2,"editRange":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}}},"items":[{"label":"print","textEditText":"print($0)","kind":3,"data":{"id":1}}]}}]).to_string());
    e.key(KeyEvent::from(KeyCode::Tab));
    e.restore_preview();
    e.cmp.resolve_due = false;
    e.receive_lsp(&json!([{"method":"completionItem/resolve","result":{"documentation":{"kind":"markdown","value":"# Print\n**Output** text\n```rust\nfn print(value: String)\n```"}}}]).to_string());
    let backend = ratatui::backend::TestBackend::new(120, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| e.draw_completion(f, ratatui::layout::Rect::new(0, 0, 120, 28)))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text = buffer
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("print"));
    assert!(text.contains("Output"));
    if let Some(path) = std::env::var_os("LEAFVIEW_COMPLETION_RENDER") {
        let rows: Vec<String> = buffer
            .content
            .chunks(120)
            .map(|row| row.iter().map(|c| c.symbol()).collect())
            .collect();
        std::fs::write(path, serde_json::to_string(&rows).unwrap()).unwrap();
    }
    for (w, h) in [(8, 5), (20, 8), (40, 10)] {
        let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        t.draw(|f| e.draw_completion(f, ratatui::layout::Rect::new(0, 0, w, h)))
            .unwrap();
    }
}
#[test]
fn browse_never_edits_and_server_refresh_preserves_selection() {
    let mut e = editor();
    e.buffers[0] = Buffer::from_text("/tmp/main.typ".into(), Rope::from_str("#page"));
    e.buffers[0].set_cursor(5);
    let items = json!([
        {"label":"import (package)","filterText":"import","sortText":"0"},
        {"label":"include (package)","sortText":"1"},
        {"label":"pagebreak","sortText":"3"},
        {"label":"page","sortText":"2"}
    ]);
    show(&mut e, items.clone());
    let (_, list, _) = e.popup.as_ref().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["label"], "page");
    let version = e.buffers[0].version;
    e.key(KeyEvent::from(KeyCode::Tab));
    e.key(KeyEvent::from(KeyCode::Down));
    assert_eq!(e.buffers[0].state.doc.to_string(), "#page");
    assert_eq!(e.buffers[0].version, version);
    assert!(!e.buffers[0].dirty());
    show(&mut e, items);
    let (_, list, i) = e.popup.as_ref().unwrap();
    assert_eq!(list[*i]["label"], "pagebreak");
    e.key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(e.buffers[0].state.doc.to_string(), "#pagebreak");
}
#[test]
fn late_resolve_cannot_change_or_accept_another_item() {
    let mut e = editor();
    show(
        &mut e,
        json!([{"label":"print","data":{"id":1}},{"label":"println","data":{"id":2}}]),
    );
    e.key(KeyEvent::from(KeyCode::Tab));
    let old = e.popup.as_ref().unwrap().1[0].clone();
    e.key(KeyEvent::from(KeyCode::Down));
    e.cmp.resolve_due = false;
    e.cmp.resolving_item = Some(old);
    e.resolved_completion(&json!({"label":"print","insertText":"WRONG"}));
    assert_eq!(e.popup.as_ref().unwrap().1[1]["label"], "println");
    assert!(!e.cmp.resolved);
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri");
}
#[test]
fn filter_text_and_fuzzy_before_truncation() {
    let mut e = editor();
    let mut items = (0..300)
        .map(|i| json!({"label":format!("unrelated{i}")}))
        .collect::<Vec<_>>();
    items.push(json!({"label":"Print display","filterText":"print"}));
    items.push(json!({"label":"parse_request_id"}));
    show(&mut e, json!(items));
    let (_, items, _) = e.popup.as_ref().unwrap();
    assert_eq!(items[0]["label"], "Print display");
    assert!(items.iter().any(|i| i["label"] == "parse_request_id"));
}
#[test]
fn paired_calls_skip_close_and_delete_pair() {
    let mut e = editor();
    e.key(KeyEvent::from(KeyCode::Char('(')));
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri()");
    assert_eq!(e.buffers[0].cursor(), 4);
    assert!(e.signature_due.is_some());
    e.key(KeyEvent::from(KeyCode::Char(')')));
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri()");
    assert!(e.signature_due.is_none());
    e.key(KeyEvent::from(KeyCode::Left));
    e.key(KeyEvent::from(KeyCode::Backspace));
    assert_eq!(e.buffers[0].state.doc.to_string(), "pri");
}
#[test]
fn signature_context_and_active_parameter_utf16() {
    for (text, arg, name) in [
        ("f(1, g(2, 3), ", 2, None),
        ("f(\"a,b\", ", 1, None),
        ("f([1,2], key: ", 1, Some("key")),
        ("f(1, /* , */ ", 1, None),
    ] {
        let rope = Rope::from_str(text);
        let call = signature::context_for(&rope, rope.len_chars(), false).unwrap();
        assert_eq!(call.argument, arg, "{text}");
        assert_eq!(call.name.as_deref(), name);
    }
    assert!(signature::context_for(&Rope::from_str("f(1)"), 4, false).is_none());
    let call = signature::Call {
        open: 1,
        argument: 1,
        name: None,
    };
    let v = json!({"activeSignature":1,"activeParameter":0,"signatures":[{"label":"wrong()"},{"label":"f(😀, value)","activeParameter":1,"parameters":[{"label":[2,4]},{"label":[6,11],"documentation":"The value"}]}]});
    let (label, range, doc) = signature::active_range(&v, &call).unwrap();
    assert_eq!(&label[range], "value");
    assert!(doc.contains("The value"));
}
#[test]
fn signature_coexists_and_updates_while_typing() {
    let mut e = editor();
    e.buffers[0] = Buffer::from_text("/tmp/main.typ".into(), Rope::from_str("#page("));
    e.buffers[0].set_cursor(6);
    e.schedule_signature();
    assert!(e.signature_due.is_some());
    e.receive_lsp(&json!([{"method":"textDocument/signatureHelp","result":{"signatures":[{"label":"page(width: length, height: length)","parameters":[{"label":"width: length"},{"label":"height: length"}]}]}}]).to_string());
    e.key(KeyEvent::from(KeyCode::Char('w')));
    assert!(e.signature_due.is_some());
    assert_eq!(e.auto_due.as_ref().unwrap().1, "complete");
    show(&mut e, json!([{"label":"width"}]));
    e.key(KeyEvent::from(KeyCode::Tab));
    assert!(!e.signature.value.is_null());
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 100, 24);
            e.draw_signature(f, area);
            e.draw_completion(f, area);
        })
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("Signature"));
    assert!(text.contains("width"));
    if let Some(path) = std::env::var_os("LEAFVIEW_SIGNATURE_RENDER") {
        let cells=terminal.backend().buffer().content.iter().map(|cell|json!({"s":cell.symbol(),"fg":format!("{:?}",cell.fg),"bg":format!("{:?}",cell.bg),"bold":cell.modifier.contains(ratatui::style::Modifier::BOLD),"underline":cell.modifier.contains(ratatui::style::Modifier::UNDERLINED)})).collect::<Vec<_>>();
        std::fs::write(
            path,
            serde_json::to_string(&json!({"width":100,"height":24,"cells":cells})).unwrap(),
        )
        .unwrap();
    }
    e.key(KeyEvent::from(KeyCode::Char(')')));
    assert!(e.signature.value.is_null());
    assert!(e.signature_due.is_none());
}
#[test]
fn hover_is_markdown_not_a_completion_list_and_windows_close() {
    let mut e = editor();
    e.receive_lsp(&json!([{"method":"textDocument/hover","result":{"contents":{"kind":"markdown","value":"```typst\n#let v(amount: length) = none\n```\n\nInserts spacing.\n\n## Example\n#v(1fr)"}}}]).to_string());
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 24)).unwrap();
    terminal
        .draw(|f| e.draw_completion(f, ratatui::layout::Rect::new(0, 0, 100, 24)))
        .unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("Inserts spacing."));
    assert!(!text.contains('›'));
    assert!(!text.contains("```"));
    if let Some(path) = std::env::var_os("LEAFVIEW_HOVER_RENDER") {
        let buffer = terminal.backend().buffer();
        let rows = buffer
            .content
            .chunks(100)
            .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
            .collect::<Vec<_>>();
        std::fs::write(path, serde_json::to_string(&rows).unwrap()).unwrap();
    }
    let before = e.buffers[0].version;
    e.key(KeyEvent::from(KeyCode::F(7)));
    assert_eq!(e.windows.focus, Some(windows::Window::Documentation));
    e.key(KeyEvent::from(KeyCode::PageDown));
    assert_eq!(e.windows.hover_scroll, 6);
    assert_eq!(e.buffers[0].version, before);
    e.key(KeyEvent::from(KeyCode::Esc));
    assert!(e.windows.focus.is_none());
    assert!(e.popup.is_some());
    e.key(KeyEvent::from(KeyCode::F(8)));
    assert!(e.popup.is_none());
}
#[test]
fn tabs_close_button_and_unsaved_guard() {
    let mut e = editor();
    e.buffers.push(Buffer::from_text(
        "/tmp/second.rs".into(),
        Rope::from_str("hello"),
    ));
    let area = ratatui::layout::Rect::new(0, 0, 100, 24);
    let (_, x, width) = e.tab_regions(area)[1];
    e.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x + width - 2,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(e.buffers.len(), 1);
    e.buffers[0].edit(0, 0, "x");
    e.key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert_eq!(e.buffers.len(), 1);
    assert!(e.message.contains("Unsaved"));
}
#[test]
fn local_sources_and_config() {
    let config: completion::Config =
        serde_json::from_str(include_str!("../../examples/completion.json")).unwrap();
    assert!(!config.ghost_text);
    assert!(serde_json::from_str::<completion::Config>(r#"{"typo":true}"#).is_err());
    let mut e = editor();
    e.buffers[0] = Buffer::from_text("/tmp/main.py".into(), Rope::from_str("def"));
    e.buffers[0].set_cursor(3);
    let candidates = completion_sources::local(&e.buffers[0], &config);
    assert!(candidates
        .iter()
        .any(|v| v["label"] == "def" && v["insertTextFormat"] == 2));
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hello.py"), "").unwrap();
    let candidates =
        completion_sources::paths(&dir.path().join("main.py"), &Rope::from_str("'./hel"), 6);
    assert!(candidates.iter().any(|v| v["label"] == "hello.py"));
}
#[test]
#[ignore = "requires tinymist"]
fn real_typst_page_filter_and_automatic_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.typ");
    std::fs::write(&path, "#page").unwrap();
    let mut e = editor();
    e.buffers[0] = Buffer::from_text(path, Rope::from_str("#pag"));
    e.buffers[0].set_cursor(4);
    e.key(KeyEvent::from(KeyCode::Char('e')));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        e.tick();
        e.poll();
        if e.completion_visible() && e.lsp_pending.is_none() {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{}", e.message);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (_, items, _) = e.popup.as_ref().unwrap();
    assert!(items
        .iter()
        .any(|i| i["label"].as_str().is_some_and(|s| s.starts_with("page"))));
    assert!(!items.iter().any(|i| i["label"]
        .as_str()
        .is_some_and(|s| s.starts_with("import") || s.starts_with("include"))));
    let before = e.buffers[0].version;
    for _ in 0..3 {
        e.key(KeyEvent::from(KeyCode::Tab));
    }
    assert_eq!(e.buffers[0].state.doc.to_string(), "#page");
    assert_eq!(e.buffers[0].version, before);
    e.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
    e.key(KeyEvent::from(KeyCode::Char('(')));
    assert_eq!(e.buffers[0].state.doc.to_string(), "#page()");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while e.signature.value["signatures"]
        .as_array()
        .is_none_or(|s| s.is_empty())
    {
        e.tick();
        e.poll();
        assert!(std::time::Instant::now() < deadline, "{}", e.message);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let call =
        signature::context_for(&e.buffers[0].state.doc, e.buffers[0].cursor(), false).unwrap();
    let (label, _, _) = signature::active_range(&e.signature.value, &call).unwrap();
    assert!(label.contains("page"));
}
#[test]
#[ignore = "requires rust-analyzer, basedpyright-langserver and tinymist"]
fn real_three_language_signature() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='signature_fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    for (file, text, needle) in [
        (
            "src/main.rs",
            "fn greet(name: &str, count: usize) {}\nfn main() { greet(\"a\", ",
            "count",
        ),
        (
            "main.py",
            "def greet(name: str, count: int):\n    pass\ngreet(\"a\", ",
            "count",
        ),
        (
            "main.typ",
            "#let greet(name, count) = [#name]\n#greet(\"a\", ",
            "count",
        ),
    ] {
        let path = dir.path().join(file);
        let cursor = text.chars().count();
        let text = if file.ends_with("typ") {
            format!("{text})")
        } else {
            text.to_owned()
        };
        std::fs::write(&path, &text).unwrap();
        let mut runner = tasks::Runner::default();
        let stop = std::sync::atomic::AtomicBool::new(false);
        let mut found = false;
        for _ in 0..10 {
            let Reply::Lsp(_, _, reply) = runner
                .run(
                    Task::Lsp(
                        path.clone(),
                        1,
                        Rope::from_str(&text),
                        cursor,
                        "signature".into(),
                    ),
                    &stop,
                )
                .unwrap()
            else {
                panic!()
            };
            let messages: serde_json::Value = serde_json::from_str(&reply).unwrap();
            assert!(messages[0]["error"].is_null(), "{file}: {reply}");
            if messages[0]["result"]["signatures"]
                .as_array()
                .is_some_and(|s| !s.is_empty())
            {
                let call =
                    signature::context_for(&Rope::from_str(&text), cursor, file.ends_with("py"))
                        .unwrap();
                let (label, range, _) =
                    signature::active_range(&messages[0]["result"], &call).unwrap();
                assert!(label.contains(needle), "{file}: {reply}");
                assert!(
                    label[range].contains(needle),
                    "{file}: wrong active parameter: {reply}"
                );
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
        assert!(found, "{file}: no signature");
    }
}
#[test]
#[ignore = "requires rust-analyzer, basedpyright-langserver and tinymist"]
fn real_three_language_completion() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='completion_fixture'\nversion='0.1.0'\nedition='2021'\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    for (file, text, prefix) in [
        (
            "src/main.rs",
            "fn main() { let greeting = 1; let _x = greet",
            "greeting",
        ),
        ("main.py", "greeting = 1\ngreet", "greeting"),
        ("main.typ", "#let greeting = 1\n#greet", "greeting"),
    ] {
        let path = dir.path().join(file);
        std::fs::write(&path, text).unwrap();
        let mut runner = tasks::Runner::default();
        let stop = std::sync::atomic::AtomicBool::new(false);
        let mut found = false;
        for _ in 0..8 {
            let reply = runner
                .run(
                    Task::Lsp(
                        path.clone(),
                        0,
                        Rope::from_str(text),
                        text.chars().count(),
                        "complete".into(),
                    ),
                    &stop,
                )
                .unwrap();
            let Reply::Lsp(_, _, reply) = reply else {
                panic!()
            };
            let messages: serde_json::Value = serde_json::from_str(&reply).unwrap();
            assert!(messages[0]["error"].is_null(), "{file}: {reply}");
            if reply.contains(prefix) {
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(700));
        }
        assert!(found, "{file}: missing {prefix}");
    }
}
