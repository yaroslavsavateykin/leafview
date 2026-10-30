use super::*;
use helix_core::Rope;
fn buffer(text: &str) -> Buffer {
    Buffer::from_text(PathBuf::from("/tmp/test.typ"), Rope::from_str(text))
}
fn feed(v: &mut vim::Vim, b: &mut Buffer, keys: &str) {
    for c in keys.chars() {
        let code = match c {
            '\u{1b}' => KeyCode::Esc,
            '\n' => KeyCode::Enter,
            _ => KeyCode::Char(c),
        };
        v.key(b, KeyEvent::from(code));
    }
}
#[test]
fn operators_objects_history_unicode() {
    for (keys, expected) in [
        ("diw", " beta\n"),
        ("ciwλ\u{1b}", "λ beta\n"),
        ("d2w", "\n"),
        ("ggVGd", ""),
    ] {
        let mut b = buffer("alpha beta\n");
        let mut v = vim::Vim::default();
        feed(&mut v, &mut b, keys);
        assert_eq!(b.state.doc.to_string(), expected, "{keys}");
        b.undo(false);
        assert_eq!(b.state.doc.to_string(), "alpha beta\n");
        b.undo(true);
        assert_eq!(b.state.doc.to_string(), expected);
    }
}
#[test]
fn counts_register_macro_repeat_visual() {
    let mut b = buffer("one\ntwo\nthree\nfour\n");
    let mut v = vim::Vim::default();
    feed(&mut v, &mut b, "2dd");
    assert_eq!(b.state.doc.to_string(), "three\nfour\n");
    feed(&mut v, &mut b, ".");
    assert_eq!(b.state.doc.to_string(), "");
    let mut b = buffer("abc def\n");
    let mut v = vim::Vim::default();
    feed(&mut v, &mut b, "qaxq@a");
    assert_eq!(b.state.doc.to_string(), "c def\n");
    let mut b = buffer("one\ntwo\n");
    let mut v = vim::Vim::default();
    feed(&mut v, &mut b, "\"ayyGp");
    assert_eq!(b.state.doc.to_string(), "one\ntwo\none\n");
    let mut b = buffer("abc\n");
    let mut v = vim::Vim::default();
    feed(&mut v, &mut b, "vld");
    assert_eq!(b.state.doc.to_string(), "c\n");
}
#[test]
fn paragraph_search_and_crlf_save() {
    let mut b = buffer("one\r\ntwo\r\n\r\nthree\r\n");
    let mut v = vim::Vim::default();
    feed(&mut v, &mut b, "dap");
    assert_eq!(b.state.doc.to_string(), "three\r\n");
    b.undo(false);
    feed(&mut v, &mut b, "/three\n");
    assert_eq!(b.line(), 3);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.typ");
    buffer::atomic_save(&path, &b.state.doc).unwrap();
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "one\r\ntwo\r\n\r\nthree\r\n"
    );
}
#[test]
fn substitution_and_stale_formatter_rejection() {
    let mut editor = Editor::new();
    editor.format_on_save = false;
    editor.buffers.push(buffer("foo foo\nfoo\n"));
    for c in ":%s/foo/bar/g\n".chars() {
        editor.key(KeyEvent::from(if c == '\n' {
            KeyCode::Enter
        } else {
            KeyCode::Char(c)
        }));
    }
    assert_eq!(editor.buffers[0].state.doc.to_string(), "bar bar\nbar\n");
    editor.buffers[0].undo(false);
    assert_eq!(editor.buffers[0].state.doc.to_string(), "foo foo\nfoo\n");
    let original = editor.buffers[0].state.doc.clone();
    editor.buffers[0].edit(0, 0, "new");
    assert_eq!(
        original.to_string(),
        "foo foo\nfoo\n",
        "shared rope snapshot changed"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.typ");
    editor.buffers[0].path = path.clone();
    editor.save();
    editor.buffers[0].edit(0, 0, "later");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !editor.poll() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        editor.buffers[0].dirty(),
        "saving an old version marked new edits saved"
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "newfoo foo\nfoo\n");
}

#[test]
fn external_rustfmt_and_undo() {
    let mut editor = Editor::new();
    let mut b = buffer("fn main(){let x=1;}\n");
    b.path = PathBuf::from("/tmp/leafview-format-test.rs");
    editor.buffers.push(b);
    editor.format();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !editor.poll() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(editor.message, "Formatted");
    assert!(editor.buffers[0]
        .state
        .doc
        .to_string()
        .contains("let x = 1;"));
    editor.buffers[0].undo(false);
    assert_eq!(
        editor.buffers[0].state.doc.to_string(),
        "fn main(){let x=1;}\n"
    );
}
#[test]
fn editor_render_snapshot_and_tiny_sizes() {
    let mut editor = Editor::new();
    editor
        .buffers
        .push(buffer("#set text(size: 12pt)\nHello λ and 世界\n"));
    editor.visible = true;
    editor.focused = true;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(70, 12)).unwrap();
    terminal.draw(|f| editor.render(f, f.area())).unwrap();
    let buffer = terminal.backend().buffer();
    let snapshot = (0..12)
        .map(|y| (0..70).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(snapshot.contains("Hello λ"));
    assert!(snapshot.lines().next().unwrap().contains("test.typ"));
    assert!(!snapshot.lines().next().unwrap().contains("Normal"));
    assert!(snapshot.lines().nth(10).unwrap().contains("Normal  1:1"));
    assert_eq!(buffer[(7, 2)].fg, super::syntax::Token::Keyword.color());
    if let Ok(path) = std::env::var("LEAFVIEW_EDITOR_SNAPSHOT") {
        std::fs::write(path, snapshot).unwrap();
    }
    for width in 0..10 {
        terminal
            .draw(|f| editor.render(f, ratatui::layout::Rect::new(0, 0, width, 3)))
            .unwrap();
    }
}
#[test]
fn completion_resize_and_command_hints() {
    let mut editor = Editor::new();
    editor.buffers.push(buffer("hello helium\nhe\n"));
    editor.buffers[0].set_cursor(15);
    editor.vim.mode = vim::Mode::Insert;
    // Ctrl-N now opens completion rather than immediately replacing a word.
    editor.complete_word(false);
    assert!(editor.buffers[0].state.doc.to_string().contains("helium\n"));
    editor.complete_word(false);
    assert!(editor.buffers[0].state.doc.to_string().ends_with("hello\n"));
    editor.key(KeyEvent::from(KeyCode::Esc));
    editor.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    editor.key(KeyEvent::from(KeyCode::Char('>')));
    assert_eq!(editor.ratio, 55);
    for _ in 0..20 {
        editor.key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
    }
    assert_eq!(editor.ratio, 80);
    editor.key(KeyEvent::from(KeyCode::Char('d')));
    assert!(editor.vim.message.contains("motion"));
}

#[test]
fn mouse_insert_arrows_and_format_on_save() {
    let mut editor = Editor::new();
    editor.buffers.push(buffer("one\ntwo\nthree\n"));
    let area = ratatui::layout::Rect::new(20, 0, 60, 10);
    editor.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 28,
            row: 3,
            modifiers: KeyModifiers::NONE,
        },
        area,
    );
    assert_eq!(editor.buffers[0].line(), 1);
    assert_eq!(editor.buffers[0].cursor(), 6);
    editor.vim.mode = vim::Mode::Insert;
    editor.key(KeyEvent::from(KeyCode::Left));
    assert_eq!(editor.buffers[0].cursor(), 5);
    editor.key(KeyEvent::from(KeyCode::End));
    assert_eq!(editor.buffers[0].cursor(), 7);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.rs");
    editor.buffers[0] = Buffer::from_text(path.clone(), Rope::from_str("fn main(){let x=1;}\n"));
    editor.save();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        editor.poll();
        if !editor.busy {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(std::fs::read_to_string(path)
        .unwrap()
        .contains("let x = 1;"));
    assert!(!editor.dirty());
}

#[test]
fn wheel_scroll_and_stale_format_on_save() {
    let mut editor = Editor::new();
    let text = "line\n".repeat(100);
    editor.buffers.push(buffer(&text));
    editor.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 30,
            row: 5,
            modifiers: KeyModifiers::NONE,
        },
        ratatui::layout::Rect::new(20, 0, 60, 10),
    );
    assert_eq!(editor.buffers[0].scroll, 3);
    assert_eq!(editor.buffers[0].line(), 3);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.rs");
    std::fs::write(&path, "sentinel").unwrap();
    editor.buffers[0] = Buffer::from_text(path.clone(), Rope::from_str("fn main(){}\n"));
    editor.save();
    editor.buffers[0].edit(0, 0, "// new edit\n");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        editor.poll();
        if !editor.busy {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(std::fs::read_to_string(path).unwrap(), "sentinel");
    assert!(editor.message.contains("save again"));
}

#[test]
fn compare_neovim() {
    for keys in [
        "diw",
        "ciwNEW\u{1b}",
        "2dd",
        "d3w",
        "vld",
        "Vjd",
        "ggVGd",
        "dap",
    ] {
        let initial = "one two three\nfour five\nsix seven\n\nlast\n";
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        std::fs::write(&input, initial).unwrap();
        let lua=format!("local k=vim.api.nvim_replace_termcodes({},true,false,true); vim.cmd.normal({{args={{k}},bang=true}}); vim.fn.writefile({{vim.fn.line('.'),vim.fn.col('.')}},'{}'); vim.cmd('write! {}'); vim.cmd('qa!')",format!("\"{}\"", keys.replace('\u{1b}', "<Esc>")),dir.path().join("cursor").display(),output.display());
        let script = dir.path().join("test.lua");
        std::fs::write(&script, lua).unwrap();
        let status = std::process::Command::new("timeout")
            .args(["5", "nvim"])
            .args(["--headless", "-u", "NONE", "-n", "-i", "NONE"])
            .arg(&input)
            .arg("-c")
            .arg(format!("luafile {}", script.display()))
            .output();
        let Ok(status) = status else {
            return;
        };
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let mut b = buffer(initial);
        let mut v = vim::Vim::default();
        feed(&mut v, &mut b, keys);
        let ours = b.state.doc.to_string();
        assert_eq!(ours, std::fs::read_to_string(output).unwrap(), "{keys}");
        let expected = std::fs::read_to_string(dir.path().join("cursor")).unwrap();
        let row = b.line();
        let column = b.state.doc.slice(b.start(row)..b.cursor()).len_bytes() + 1;
        assert_eq!(
            format!("{}\n{}\n", row + 1, column),
            expected,
            "cursor: {keys}"
        );
    }
}
#[test]
#[ignore = "large file benchmark; run release with --ignored --nocapture"]
fn benchmark_large_files() {
    use std::time::Instant;
    for mb in [1, 10, 100] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.typ");
        let line = "alpha beta gamma delta epsilon\n";
        let count = mb * 1024 * 1024 / line.len();
        {
            use std::io::Write;
            let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
            for _ in 0..count {
                file.write_all(line.as_bytes()).unwrap();
            }
        }
        let t = Instant::now();
        let rope = Rope::from_reader(std::io::BufReader::new(std::fs::File::open(&path).unwrap()))
            .unwrap();
        let open = t.elapsed();
        let mut b = Buffer::from_text(path, rope);
        let mut v = vim::Vim::default();
        let mut times = vec![];
        for _ in 0..1000 {
            let t = Instant::now();
            feed(&mut v, &mut b, "ix\u{1b}");
            times.push(t.elapsed().as_nanos());
        }
        times.sort();
        let mut editor = Editor::new();
        editor.buffers.push(b);
        editor.visible = true;
        editor.focused = true;
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 40)).unwrap();
        let t = Instant::now();
        for _ in 0..100 {
            terminal.draw(|f| editor.render(f, f.area())).unwrap();
        }
        let draw = t.elapsed() / 100;
        let ram = std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .unwrap()
            .to_owned();
        println!(
            "{mb} MiB open_ms={:.3} edit_triplet_p50_us={:.3} p99_us={:.3} render_us={:.3} {ram}",
            open.as_secs_f64() * 1000.,
            times[500] as f64 / 1000.,
            times[990] as f64 / 1000.,
            draw.as_secs_f64() * 1e6
        );
    }
}
