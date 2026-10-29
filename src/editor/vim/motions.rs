use crate::editor::buffer::Buffer;
pub fn class(c: char, big: bool) -> u8 {
    if c.is_whitespace() {
        0
    } else if big || c.is_alphanumeric() || c == '_' {
        1
    } else {
        2
    }
}
pub fn motion(b: &Buffer, key: char, count: usize) -> Option<usize> {
    let doc = &b.state.doc;
    let len = doc.len_chars();
    let mut p = b.cursor();
    for _ in 0..count.min(100000) {
        let row = doc.char_to_line(p);
        let col = p - b.start(row);
        p = match key {
            'h' => {
                helix_core::graphemes::prev_grapheme_boundary(doc.slice(..), p).max(b.start(row))
            }
            'l' => helix_core::graphemes::next_grapheme_boundary(doc.slice(..), p)
                .min(b.end(row).saturating_sub(1).max(b.start(row))),
            'j' => {
                let r = (row + 1).min(doc.len_lines().saturating_sub(1));
                (b.start(r) + col).min(b.end(r).saturating_sub(1).max(b.start(r)))
            }
            'k' => {
                let r = row.saturating_sub(1);
                (b.start(r) + col).min(b.end(r).saturating_sub(1).max(b.start(r)))
            }
            '0' => b.start(row),
            '$' => b.end(row).saturating_sub(1).max(b.start(row)),
            '^' => (b.start(row)..b.end(row))
                .find(|&i| !doc.char(i).is_whitespace())
                .unwrap_or(b.start(row)),
            'w' | 'W' => {
                if p < len {
                    let c = class(doc.char(p), key == 'W');
                    while p < len && class(doc.char(p), key == 'W') == c {
                        p += 1;
                    }
                    while p < len && doc.char(p).is_whitespace() {
                        p += 1;
                    }
                }
                p
            }
            'b' | 'B' => {
                p = p.saturating_sub(1);
                while p > 0 && doc.char(p).is_whitespace() {
                    p -= 1;
                }
                if p < len {
                    let c = class(doc.char(p), key == 'B');
                    while p > 0 && class(doc.char(p - 1), key == 'B') == c {
                        p -= 1;
                    }
                }
                p
            }
            'e' | 'E' => {
                p = (p + 1).min(len);
                while p < len && doc.char(p).is_whitespace() {
                    p += 1;
                }
                if p < len {
                    let c = class(doc.char(p), key == 'E');
                    while p + 1 < len && class(doc.char(p + 1), key == 'E') == c {
                        p += 1;
                    }
                }
                p
            }
            '{' => {
                let mut r = row.saturating_sub(1);
                while r > 0 && b.start(r) == b.end(r) {
                    r -= 1;
                }
                while r > 0 && b.start(r) != b.end(r) {
                    r -= 1;
                }
                b.start(r)
            }
            '}' => {
                let mut r = (row + 1).min(doc.len_lines() - 1);
                while r + 1 < doc.len_lines() && b.start(r) == b.end(r) {
                    r += 1;
                }
                while r + 1 < doc.len_lines() && b.start(r) != b.end(r) {
                    r += 1;
                }
                b.start(r)
            }
            _ => return None,
        };
    }
    Some(p.min(len))
}
pub fn object(b: &Buffer, key: char, around: bool) -> Option<(usize, usize)> {
    let d = &b.state.doc;
    let n = d.len_chars();
    let p = b.cursor().min(n.saturating_sub(1));
    if n == 0 {
        return None;
    }
    if matches!(key, 'w' | 'W') {
        let c = class(d.char(p), key == 'W');
        let (mut a, mut z) = (p, p + 1);
        while a > 0 && class(d.char(a - 1), key == 'W') == c {
            a -= 1;
        }
        while z < n && class(d.char(z), key == 'W') == c {
            z += 1;
        }
        if around {
            while z < n && d.char(z).is_whitespace() {
                z += 1;
            }
        }
        return Some((a, z));
    }
    if key == 'p' {
        let mut a = d.char_to_line(p);
        let mut z = a;
        while a > 0 && b.start(a - 1) != b.end(a - 1) {
            a -= 1;
        }
        while z + 1 < d.len_lines() && b.start(z + 1) != b.end(z + 1) {
            z += 1;
        }
        if around {
            while z + 1 < d.len_lines() && b.start(z + 1) == b.end(z + 1) {
                z += 1;
            }
        }
        return Some((
            b.start(a),
            if z + 1 < d.len_lines() {
                b.start(z + 1)
            } else {
                n
            },
        ));
    }
    let (open, close) = match key {
        '(' | ')' | 'b' => ('(', ')'),
        '{' | '}' | 'B' => ('{', '}'),
        '[' | ']' => ('[', ']'),
        '"' => ('"', '"'),
        '\'' => ('\'', '\''),
        '`' => ('`', '`'),
        _ => return None,
    };
    let mut a = p;
    let mut depth = 0;
    loop {
        let c = d.char(a);
        if c == open {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if c == close {
            depth += 1;
        }
        if a == 0 {
            return None;
        }
        a -= 1;
    }
    let mut z = a + 1;
    depth = 0;
    while z < n {
        let c = d.char(z);
        if c == close {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if c == open {
            depth += 1;
        }
        z += 1;
    }
    if z == n {
        return None;
    }
    Some(if around { (a, z + 1) } else { (a + 1, z) })
}
