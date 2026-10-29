//! Bounded viewport lexical highlighting. No whole-Rope scan on edits or draws.
//! This deliberately does not claim semantic or Tree-sitter highlighting.
use ratatui::style::Color;
use std::path::Path;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Plain,
    Comment,
    String,
    Number,
    Keyword,
    Function,
    Markup,
    Operator,
}
impl Token {
    pub fn color(self) -> Color {
        match self {
            Self::Plain => Color::Rgb(226, 233, 244),
            Self::Comment => Color::Rgb(137, 159, 180),
            Self::String => Color::Rgb(183, 230, 143),
            Self::Number => Color::Rgb(255, 205, 123),
            Self::Keyword => Color::Rgb(216, 174, 255),
            Self::Function => Color::Rgb(133, 202, 255),
            Self::Markup => Color::Rgb(110, 205, 192),
            Self::Operator => Color::Rgb(180, 192, 211),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Typst,
    Python,
    Code,
    Plain,
}
fn language(path: &Path) -> Language {
    match path.extension().and_then(|x| x.to_str()).unwrap_or("") {
        "typ" => Language::Typst,
        "py" => Language::Python,
        "rs" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" | "json" => Language::Code,
        _ => Language::Plain,
    }
}
pub struct SyntaxCache {
    lines: lru::LruCache<(u64, usize, usize), Vec<Token>>,
}
impl Default for SyntaxCache {
    fn default() -> Self {
        Self {
            lines: lru::LruCache::new(std::num::NonZeroUsize::new(256).unwrap()),
        }
    }
}
impl SyntaxCache {
    pub fn line(
        &mut self,
        path: &Path,
        version: u64,
        row: usize,
        text: helix_core::RopeSlice<'_>,
        limit: usize,
    ) -> &[Token] {
        let key = (version, row, limit);
        if !self.lines.contains(&key) {
            let chars: Vec<char> = text.chars().take(limit.min(4096)).collect();
            self.lines.put(key, highlight(&chars, language(path)));
        }
        self.lines.get(&key).unwrap()
    }
}
fn highlight(chars: &[char], language: Language) -> Vec<Token> {
    let mut out = vec![Token::Plain; chars.len()];
    if language == Language::Plain {
        return out;
    }
    let mut i = 0;
    let mut code = language != Language::Typst;
    while i < chars.len() {
        let c = chars[i];
        if (c == '/' && chars.get(i + 1) == Some(&'/'))
            || (c == '#' && language == Language::Python)
        {
            out[i..].fill(Token::Comment);
            break;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            let a = i;
            i += 2;
            while i < chars.len() {
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    i += 2;
                    break;
                }
                i += 1;
            }
            out[a..i.min(chars.len())].fill(Token::Comment);
            continue;
        }
        if c == '"' || (c == '\'' && language != Language::Typst) || c == '`' {
            let a = i;
            let quote = c;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i = (i + 2).min(chars.len());
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out[a..i].fill(Token::String);
            continue;
        }
        if language == Language::Typst {
            if c == '#' {
                out[i] = Token::Operator;
                code = true;
                i += 1;
                continue;
            }
            if c == '=' && chars[..i].iter().all(|c| c.is_whitespace() || *c == '=') {
                out[i..].fill(Token::Markup);
                break;
            }
            if matches!(c, '$' | '@' | '<' | '>') {
                out[i] = Token::Markup;
            }
        }
        if c.is_ascii_digit() && (code || language != Language::Typst) {
            let a = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '.' | '_'))
            {
                i += 1;
            }
            out[a..i].fill(Token::Number);
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let a = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || (language == Language::Typst && chars[i] == '-'))
            {
                i += 1;
            }
            let word: String = chars[a..i].iter().collect();
            let token = if matches!(
                word.as_str(),
                "let"
                    | "set"
                    | "show"
                    | "if"
                    | "else"
                    | "for"
                    | "in"
                    | "while"
                    | "return"
                    | "break"
                    | "continue"
                    | "import"
                    | "include"
                    | "as"
                    | "true"
                    | "false"
                    | "none"
                    | "auto"
                    | "fn"
                    | "pub"
                    | "use"
                    | "mod"
                    | "struct"
                    | "enum"
                    | "impl"
                    | "trait"
                    | "match"
                    | "mut"
                    | "const"
                    | "static"
                    | "async"
                    | "await"
                    | "def"
                    | "class"
                    | "from"
                    | "try"
                    | "except"
                    | "with"
                    | "pass"
                    | "None"
                    | "True"
                    | "False"
                    | "function"
                    | "export"
                    | "default"
                    | "var"
                    | "int"
                    | "void"
                    | "bool"
                    | "char"
            ) {
                if code {
                    Token::Keyword
                } else {
                    Token::Plain
                }
            } else if code && chars[i..].iter().find(|c| !c.is_whitespace()) == Some(&'(') {
                Token::Function
            } else {
                Token::Plain
            };
            out[a..i].fill(token);
            if language == Language::Typst && a > 0 && chars[a - 1] == '#' {
                out[a - 1] = if token == Token::Plain {
                    Token::Function
                } else {
                    token
                };
            }
            continue;
        }
        if code && "(){}[],:;=+-*/!&|".contains(c) {
            out[i] = Token::Operator;
        }
        i += 1;
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typst_tokens_and_unicode() {
        let text: Vec<_> = "#let λ = 12 // comment".chars().collect();
        let out = highlight(&text, Language::Typst);
        assert_eq!(out[1], Token::Keyword);
        assert_eq!(out[9], Token::Number);
        assert_eq!(*out.last().unwrap(), Token::Comment);
        let text: Vec<_> = "Hello world".chars().collect();
        assert!(highlight(&text, Language::Typst)
            .iter()
            .all(|t| *t == Token::Plain));
    }
    #[test]
    fn cache_bounded_and_invalidated() {
        let mut cache = SyntaxCache::default();
        let text = helix_core::Rope::from_str("#let x = \"hello\"");
        for version in 0..1000 {
            assert_eq!(
                cache.line(Path::new("a.typ"), version, 0, text.slice(..), 100)[1],
                Token::Keyword
            );
        }
        assert_eq!(cache.lines.len(), 256);
        assert!(
            cache
                .line(Path::new("a.typ"), 1001, 0, text.slice(..), 3)
                .len()
                <= 3
        );
    }
}
