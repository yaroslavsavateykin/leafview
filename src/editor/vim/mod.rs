mod motions;
use super::buffer::Buffer;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Replace,
    Command,
}
#[derive(Clone, Copy)]
enum Pending {
    None,
    Operator(char, usize),
    Object(char, bool, usize),
    G,
    Indent(char),
    Register,
    Record,
    Play,
    Replace,
    Find(char),
}
#[derive(Default, Clone)]
struct Register {
    text: String,
    line: bool,
}
pub struct Vim {
    pub mode: Mode,
    pub command: String,
    pub message: String,
    pending: Pending,
    count: usize,
    anchor: usize,
    register: char,
    registers: HashMap<char, Register>,
    macros: HashMap<char, Vec<KeyEvent>>,
    recording: Option<char>,
    recorded: Vec<KeyEvent>,
    last_macro: char,
    replaying: bool,
    change: Vec<KeyEvent>,
    last_change: Vec<KeyEvent>,
    search: String,
    backwards: bool,
    last_find: Option<(char, char)>,
    pub mappings: HashMap<char, Vec<KeyEvent>>,
}
impl Default for Vim {
    fn default() -> Self {
        Self {
            mode: Mode::Normal,
            command: String::new(),
            message: String::new(),
            pending: Pending::None,
            count: 0,
            anchor: 0,
            register: '"',
            registers: HashMap::new(),
            macros: HashMap::new(),
            recording: None,
            recorded: vec![],
            last_macro: 'a',
            replaying: false,
            change: vec![],
            last_change: vec![],
            search: String::new(),
            backwards: false,
            last_find: None,
            mappings: HashMap::new(),
        }
    }
}
impl Vim {
    fn visual(&self) -> bool {
        matches!(
            self.mode,
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock
        )
    }
    fn put_register(&mut self, text: String, line: bool, yank: bool) {
        let r = Register { text, line };
        self.registers.insert(self.register, r.clone());
        self.registers.insert('"', r.clone());
        if yank {
            self.registers.insert('0', r);
        }
        self.register = '"';
    }
    fn operate(&mut self, b: &mut Buffer, op: char, a: usize, z: usize, line: bool) {
        let a = a.min(b.state.doc.len_chars());
        let z = z.min(b.state.doc.len_chars()).max(a);
        self.put_register(b.state.doc.slice(a..z).to_string(), line, op == 'y');
        if op != 'y' {
            b.edit(a, z, "");
            b.set_cursor(a);
            if op == 'c' {
                self.mode = Mode::Insert;
            } else {
                b.commit();
                self.mode = Mode::Normal;
            }
        } else {
            b.set_cursor(a);
            self.mode = Mode::Normal;
        }
        self.pending = Pending::None;
    }
    fn lines(&mut self, b: &mut Buffer, op: char, n: usize) {
        let a = b.start(b.line());
        let row = (b.line() + n).min(b.state.doc.len_lines());
        let z = if row == b.state.doc.len_lines() {
            b.state.doc.len_chars()
        } else {
            b.start(row)
        };
        self.operate(b, op, a, z, true);
        if op == 'c' {
            b.edit(a, a, "\n");
            b.set_cursor(a);
        }
    }
    pub fn key(&mut self, b: &mut Buffer, key: KeyEvent) -> Option<String> {
        self.message.clear();
        let before = b.version;
        let was = self.mode;
        if self.recording.is_some()
            && !(self.mode == Mode::Normal && key.code == KeyCode::Char('q'))
            && !self.replaying
        {
            self.recorded.push(key);
        }
        if !self.replaying {
            self.change.push(key);
        }
        let action = self.dispatch(b, key);
        if !self.replaying && self.mode == Mode::Normal {
            if b.version != before || matches!(was, Mode::Insert | Mode::Replace) {
                self.last_change = std::mem::take(&mut self.change);
            } else if matches!(self.pending, Pending::None) && self.count == 0 {
                self.change.clear();
            }
        }
        if self.message.is_empty() {
            self.message = match self.pending {
                Pending::Operator(op, _) => format!("{op}…  motion / iw / aw / i\" / i("),
                Pending::Object(_, around, _) => format!(
                    "{}…  w word · p paragraph · \" string · ( block",
                    if around { "a" } else { "i" }
                ),
                Pending::G => "g…  g first line".into(),
                Pending::Find(_) => "Find: type a character".into(),
                Pending::Register => "Register: a–z / 0 / unnamed".into(),
                Pending::Record => "Record macro: a–z".into(),
                Pending::Play => "Play macro: a–z / @ repeat".into(),
                _ => String::new(),
            };
        }
        action
    }
    fn replay(&mut self, b: &mut Buffer, keys: Vec<KeyEvent>, n: usize) {
        if self.replaying {
            return;
        }
        self.replaying = true;
        for _ in 0..n.min(1000) {
            for k in &keys {
                self.dispatch(b, *k);
            }
        }
        self.replaying = false;
    }
    fn find(&mut self, b: &mut Buffer, kind: char, c: char, n: usize) {
        let mut p = b.cursor();
        for _ in 0..n {
            let found = if matches!(kind, 'f' | 't') {
                (p + 1..b.end(b.line())).find(|&i| b.state.doc.char(i) == c)
            } else {
                (b.start(b.line())..p)
                    .rev()
                    .find(|&i| b.state.doc.char(i) == c)
            };
            if let Some(q) = found {
                p = q;
            } else {
                return;
            }
        }
        if kind == 't' {
            p = p.saturating_sub(1);
        }
        if kind == 'T' {
            p += 1;
        }
        b.set_cursor(p);
        self.last_find = Some((kind, c));
    }
    fn search(&mut self, b: &mut Buffer, back: bool) {
        if self.search.is_empty() {
            return;
        }
        let needle: Vec<char> = self.search.chars().collect();
        let d = &b.state.doc;
        let len = d.len_chars();
        let p = b.cursor();
        let matched = |i: usize| {
            i + needle.len() <= len
                && d.slice(i..i + needle.len())
                    .chars()
                    .eq(needle.iter().copied())
        };
        let found = if back {
            (0..p).rev().chain((p..len).rev()).find(|&i| matched(i))
        } else {
            (p.saturating_add(1)..len)
                .chain(0..=p.min(len))
                .find(|&i| matched(i))
        };
        if let Some(i) = found {
            b.set_cursor(i);
        } else {
            self.message = "Pattern not found".into();
        }
    }
    fn dispatch(&mut self, b: &mut Buffer, key: KeyEvent) -> Option<String> {
        if key.code == KeyCode::Esc {
            if matches!(self.mode, Mode::Insert | Mode::Replace) {
                b.set_cursor(b.cursor().saturating_sub(1).max(b.start(b.line())));
                b.commit();
            }
            self.mode = Mode::Normal;
            self.pending = Pending::None;
            self.count = 0;
            return None;
        }
        if self.mode == Mode::Command {
            match key.code {
                KeyCode::Enter => {
                    let cmd = std::mem::take(&mut self.command);
                    self.mode = Mode::Normal;
                    if cmd.starts_with('/') || cmd.starts_with('?') {
                        self.backwards = cmd.starts_with('?');
                        self.search = cmd[1..].into();
                        self.search(b, self.backwards);
                        return None;
                    }
                    return Some(cmd.trim_start_matches(':').into());
                }
                KeyCode::Backspace => {
                    self.command.pop();
                }
                KeyCode::Char(c) => self.command.push(c),
                _ => {}
            }
            return None;
        }
        if matches!(self.mode, Mode::Insert | Mode::Replace) {
            let movement = match key.code {
                KeyCode::Left => Some('h'),
                KeyCode::Right => Some('l'),
                KeyCode::Up => Some('k'),
                KeyCode::Down => Some('j'),
                KeyCode::Home => Some('0'),
                KeyCode::End => Some('$'),
                _ => None,
            };
            if let Some(motion) = movement {
                if key.code == KeyCode::End {
                    b.set_cursor(b.end(b.line()));
                } else if key.code == KeyCode::Right {
                    b.set_cursor((b.cursor() + 1).min(b.end(b.line())));
                } else if let Some(pos) = motions::motion(b, motion, 1) {
                    b.set_cursor(pos);
                }
                return None;
            }
            match key.code {
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let p = b.cursor();
                    let end = if self.mode == Mode::Replace {
                        (p + 1).min(b.end(b.line()))
                    } else {
                        p
                    };
                    b.edit(p, end, &c.to_string());
                    b.set_cursor(p + 1);
                }
                KeyCode::Enter => {
                    let p = b.cursor();
                    b.edit(p, p, "\n");
                    b.set_cursor(p + 1);
                }
                KeyCode::Tab => {
                    let p = b.cursor();
                    b.edit(p, p, "    ");
                    b.set_cursor(p + 4);
                }
                KeyCode::Backspace => {
                    let p = b.cursor();
                    if p > 0 {
                        b.edit(p - 1, p, "");
                        b.set_cursor(p - 1);
                    }
                }
                KeyCode::Delete => {
                    let p = b.cursor();
                    if p < b.state.doc.len_chars() {
                        b.edit(p, p + 1, "");
                    }
                }
                _ => {}
            }
            return None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('r') => b.undo(true),
                KeyCode::Char('v') => {
                    self.mode = Mode::VisualBlock;
                    self.anchor = b.cursor();
                }
                _ => {}
            }
            return None;
        }
        let c = match key.code {
            KeyCode::Char(c) => c,
            KeyCode::Left => 'h',
            KeyCode::Right => 'l',
            KeyCode::Up => 'k',
            KeyCode::Down => 'j',
            _ => return None,
        };
        match self.pending {
            Pending::Register => {
                self.register = c;
                self.pending = Pending::None;
                return None;
            }
            Pending::Record => {
                self.recording = Some(c);
                self.recorded.clear();
                self.pending = Pending::None;
                return None;
            }
            Pending::Play => {
                self.pending = Pending::None;
                let r = if c == '@' { self.last_macro } else { c };
                self.last_macro = r;
                let keys = self.macros.get(&r).cloned().unwrap_or_default();
                let n = self.count.max(1);
                self.count = 0;
                self.replay(b, keys, n);
                return None;
            }
            Pending::Replace => {
                self.pending = Pending::None;
                let p = b.cursor();
                if p < b.end(b.line()) {
                    b.edit(p, p + 1, &c.to_string());
                    b.commit();
                }
                return None;
            }
            Pending::Find(kind) => {
                self.pending = Pending::None;
                let n = self.count.max(1);
                self.count = 0;
                self.find(b, kind, c, n);
                return None;
            }
            Pending::Object(op, around, n) => {
                self.pending = Pending::None;
                if let Some((a, z)) = motions::object(b, c, around) {
                    for _ in 0..n {
                        self.operate(b, op, a, z, false);
                        break;
                    }
                }
                return None;
            }
            Pending::Indent(kind) => {
                self.pending = Pending::None;
                if c == kind {
                    let row = b.line();
                    let mut edits = vec![];
                    for line in row..(row + self.count.max(1)).min(b.state.doc.len_lines()) {
                        let start = b.start(line);
                        if kind == '>' {
                            edits.push((start, start, Some("    ".into())));
                        } else if kind == '<' {
                            let mut end = start;
                            while end < (start + 4).min(b.end(line)) && b.state.doc.char(end) == ' '
                            {
                                end += 1;
                            }
                            edits.push((start, end, None));
                        } else {
                            self.message = "Use :format for syntax-aware indentation".into();
                        }
                    }
                    self.count = 0;
                    if !edits.is_empty() {
                        b.edit_many(edits);
                        b.commit();
                    }
                }
                return None;
            }
            Pending::G => {
                self.pending = Pending::None;
                if c == 'g' {
                    b.set_cursor(b.start(self.count.saturating_sub(1)));
                    self.count = 0;
                }
                return None;
            }
            _ => {}
        }
        if c.is_ascii_digit() && (c != '0' || self.count > 0) {
            self.count = (self.count * 10 + c.to_digit(10).unwrap() as usize).min(100000);
            return None;
        }
        let n = self.count.max(1);
        self.count = 0;
        if let Pending::Operator(op, outer) = self.pending {
            if c == op {
                self.lines(b, op, n * outer);
                return None;
            }
            if c == 'i' || c == 'a' {
                self.pending = Pending::Object(op, c == 'a', n * outer);
                return None;
            }
            if let Some(q) = motions::motion(b, c, n * outer) {
                let p = b.cursor();
                let inclusive = matches!(c, 'e' | 'E' | '$');
                let line = matches!(c, 'j' | 'k');
                if line {
                    let a = b.state.doc.char_to_line(p.min(q));
                    let z = b.state.doc.char_to_line(p.max(q)) + 1;
                    self.operate(
                        b,
                        op,
                        b.start(a),
                        if z < b.state.doc.len_lines() {
                            b.start(z)
                        } else {
                            b.state.doc.len_chars()
                        },
                        true,
                    );
                } else {
                    let mut end = p.max(q) + usize::from(inclusive);
                    if matches!(c, 'w' | 'W')
                        && q >= b.end(b.line())
                        && b.state.doc.char_to_line(q) > b.line()
                    {
                        end = b.end(b.line());
                    }
                    if op == 'c' && matches!(c, 'w' | 'W') {
                        while end > p && b.state.doc.char(end - 1).is_whitespace() {
                            end -= 1;
                        }
                    }
                    self.operate(b, op, p.min(q), end, false);
                }
                return None;
            }
            self.pending = Pending::None;
            self.message = "Unsupported operator motion".into();
            return None;
        }
        if let Some(q) = motions::motion(b, c, n) {
            b.set_cursor(q);
            return None;
        }
        if self.visual() && matches!(c, 'd' | 'c' | 'y' | 'x') {
            let op = if c == 'x' { 'd' } else { c };
            let p = b.cursor();
            let a = self.anchor.min(p);
            let z = (self.anchor.max(p) + 1).min(b.state.doc.len_chars());
            if self.mode == Mode::VisualLine {
                let row = b.state.doc.char_to_line(z.saturating_sub(1));
                self.operate(
                    b,
                    op,
                    b.start(b.state.doc.char_to_line(a)),
                    if row + 1 < b.state.doc.len_lines() {
                        b.start(row + 1)
                    } else {
                        b.state.doc.len_chars()
                    },
                    true,
                );
            } else if self.mode == Mode::VisualBlock {
                let ar = b.state.doc.char_to_line(self.anchor);
                let pr = b.line();
                let ac = self.anchor - b.start(ar);
                let pc = p - b.start(pr);
                let mut edits = vec![];
                let mut text = String::new();
                for r in ar.min(pr)..=ar.max(pr) {
                    let s = (b.start(r) + ac.min(pc)).min(b.end(r));
                    let e = (b.start(r) + ac.max(pc) + 1).min(b.end(r));
                    text.push_str(&b.state.doc.slice(s..e).to_string());
                    text.push('\n');
                    edits.push((s, e, None));
                }
                self.put_register(text, false, op == 'y');
                if op != 'y' {
                    b.edit_many(edits);
                    b.set_cursor(a);
                    b.commit();
                }
                self.mode = Mode::Normal;
            } else {
                self.operate(b, op, a, z, false);
            }
            return None;
        }
        match c {
            '>' | '<' | '=' => {
                self.pending = Pending::Indent(c);
                self.count = n;
            }
            '%' => {
                let d = &b.state.doc;
                let p = b.cursor();
                if p < d.len_chars() {
                    let ch = d.char(p);
                    let pair = match ch {
                        '(' => Some((')', true)),
                        '[' => Some((']', true)),
                        '{' => Some(('}', true)),
                        ')' => Some(('(', false)),
                        ']' => Some(('[', false)),
                        '}' => Some(('{', false)),
                        _ => None,
                    };
                    if let Some((other, forward)) = pair {
                        let mut depth = 0;
                        let indices: Box<dyn Iterator<Item = usize>> = if forward {
                            Box::new(p + 1..d.len_chars())
                        } else {
                            Box::new((0..p).rev())
                        };
                        let mut target = None;
                        for i in indices {
                            let c = d.char(i);
                            if c == other {
                                if depth == 0 {
                                    target = Some(i);
                                    break;
                                }
                                depth -= 1;
                            } else if c == ch {
                                depth += 1;
                            }
                        }
                        if let Some(i) = target {
                            b.set_cursor(i);
                        }
                    }
                }
            }
            'i' | 'I' | 'a' | 'A' => {
                let p = match c {
                    'I' => motions::motion(b, '^', 1).unwrap(),
                    'A' => b.end(b.line()),
                    'a' => (b.cursor() + 1).min(b.end(b.line())),
                    _ => b.cursor(),
                };
                b.set_cursor(p);
                self.mode = Mode::Insert;
            }
            'o' | 'O' => {
                let p = if c == 'o' {
                    b.end(b.line())
                } else {
                    b.start(b.line())
                };
                b.edit(p, p, "\n");
                b.set_cursor(p + usize::from(c == 'o'));
                self.mode = Mode::Insert;
            }
            'R' => self.mode = Mode::Replace,
            'r' => self.pending = Pending::Replace,
            'd' | 'c' | 'y' => self.pending = Pending::Operator(c, n),
            'D' | 'C' => {
                let p = b.cursor();
                self.operate(
                    b,
                    if c == 'D' { 'd' } else { 'c' },
                    p,
                    b.end(b.line()),
                    false,
                );
            }
            'S' => self.lines(b, 'c', n),
            'x' | 's' => {
                let p = b.cursor();
                self.operate(
                    b,
                    if c == 's' { 'c' } else { 'd' },
                    p,
                    (p + n).min(b.end(b.line())),
                    false,
                );
            }
            'X' => {
                let p = b.cursor();
                self.operate(b, 'd', p.saturating_sub(n).max(b.start(b.line())), p, false);
            }
            'p' | 'P' => {
                let r = self
                    .registers
                    .get(&self.register)
                    .cloned()
                    .unwrap_or_default();
                self.register = '"';
                let p = if r.line {
                    if c == 'p' {
                        if b.line() + 1 < b.state.doc.len_lines() {
                            b.start(b.line() + 1)
                        } else {
                            b.state.doc.len_chars()
                        }
                    } else {
                        b.start(b.line())
                    }
                } else {
                    (b.cursor() + usize::from(c == 'p')).min(b.end(b.line()))
                };
                let text = r.text.repeat(n.min(1000));
                b.edit(p, p, &text);
                b.set_cursor(p);
                b.commit();
            }
            'u' => {
                for _ in 0..n {
                    b.undo(false);
                }
            }
            'v' | 'V' => {
                self.mode = if c == 'v' {
                    Mode::Visual
                } else {
                    Mode::VisualLine
                };
                self.anchor = b.cursor();
            }
            'g' => {
                self.pending = Pending::G;
                self.count = if n > 1 { n } else { 0 };
            }
            'G' => b.set_cursor(b.start(if n > 1 {
                n - 1
            } else {
                b.state.doc.len_lines().saturating_sub(2)
            })),
            '"' => self.pending = Pending::Register,
            'q' => {
                if let Some(r) = self.recording.take() {
                    self.macros.insert(r, std::mem::take(&mut self.recorded));
                } else {
                    self.pending = Pending::Record;
                }
            }
            '@' => {
                self.pending = Pending::Play;
                self.count = n;
            }
            '.' => {
                let keys = self.last_change.clone();
                self.replay(b, keys, n);
            }
            ':' | '/' | '?' => {
                self.command = c.to_string();
                self.mode = Mode::Command;
            }
            'n' | 'N' => {
                for _ in 0..n {
                    self.search(b, self.backwards ^ (c == 'N'));
                }
            }
            '*' | '#' => {
                if let Some((a, z)) = motions::object(b, 'w', false) {
                    self.search = b.state.doc.slice(a..z).to_string();
                    self.backwards = c == '#';
                    self.search(b, self.backwards);
                }
            }
            'f' | 'F' | 't' | 'T' => {
                self.pending = Pending::Find(c);
                self.count = n;
            }
            ';' | ',' => {
                if let Some((mut k, ch)) = self.last_find {
                    if c == ',' {
                        k = match k {
                            'f' => 'F',
                            'F' => 'f',
                            't' => 'T',
                            _ => 't',
                        };
                    }
                    self.find(b, k, ch, n);
                }
            }
            'J' => {
                let row = b.line();
                if row + 1 < b.state.doc.len_lines() {
                    let a = b.end(row);
                    let mut z = b.start(row + 1);
                    while z < b.end(row + 1) && b.state.doc.char(z).is_whitespace() {
                        z += 1;
                    }
                    b.edit(a, z, " ");
                    b.set_cursor(a);
                    b.commit();
                }
            }
            _ => {
                if let Some(keys) = self.mappings.get(&c).cloned() {
                    self.replay(b, keys, n);
                } else {
                    self.message.clear();
                }
            }
        }
        None
    }
    pub fn selection(&self, b: &Buffer) -> Option<(usize, usize)> {
        self.visual()
            .then(|| (self.anchor.min(b.cursor()), self.anchor.max(b.cursor()) + 1))
    }
}
