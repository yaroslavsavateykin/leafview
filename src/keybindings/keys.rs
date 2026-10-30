//! Frontend-neutral logical/physical keystrokes; terminal events have no scan code.
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Stroke {
    pub key: String,
    pub modifiers: u8,
    pub physical: bool,
}
impl Stroke {
    pub fn event(event: KeyEvent) -> Self {
        let mut modifiers = event.modifiers.bits();
        let key = match event.code {
            KeyCode::Char(c) => {
                if !event
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
                {
                    modifiers &= !KeyModifiers::SHIFT.bits();
                }
                c.to_string()
            }
            KeyCode::Null if event.modifiers.is_empty() => {
                modifiers = KeyModifiers::CONTROL.bits();
                " ".into()
            }
            KeyCode::BackTab => {
                modifiers |= KeyModifiers::SHIFT.bits();
                "Tab".into()
            }
            KeyCode::F(n) => format!("F{n}"),
            code => format!("{code:?}"),
        };
        Self {
            key,
            modifiers,
            physical: false,
        }
    }
    pub fn parse(input: &str) -> Result<Self, String> {
        if input == "+" {
            return Ok(Self {
                key: "+".into(),
                modifiers: 0,
                physical: false,
            });
        }
        let physical = input.starts_with("Physical:");
        let input = input.strip_prefix("Physical:").unwrap_or(input);
        let input = input
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .unwrap_or(input)
            .replace("C-", "Ctrl+")
            .replace("S-", "Shift+")
            .replace("A-", "Alt+");
        let mut modifiers = KeyModifiers::NONE;
        let mut parts = input.split('+').collect::<Vec<_>>();
        let key = parts.pop().ok_or("empty key")?;
        for part in parts {
            modifiers |= match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" | "meta" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                "super" | "cmd" => KeyModifiers::SUPER,
                _ => return Err(format!("unknown modifier {part}")),
            };
        }
        let key = match key {
            "Space" => " ",
            "CR" | "Return" => "Enter",
            "Escape" => "Esc",
            "BS" => "Backspace",
            other => other,
        }
        .to_owned();
        if key.is_empty() {
            return Err("empty key".into());
        }
        Ok(Self {
            key,
            modifiers: modifiers.bits(),
            physical,
        })
    }
    pub fn label(&self) -> String {
        let mut label = if self.physical {
            "Physical:".into()
        } else {
            String::new()
        };
        for (m, s) in [
            (KeyModifiers::CONTROL, "Ctrl+"),
            (KeyModifiers::ALT, "Alt+"),
            (KeyModifiers::SHIFT, "Shift+"),
            (KeyModifiers::SUPER, "Super+"),
        ] {
            if self.modifiers & m.bits() != 0 {
                label.push_str(s);
            }
        }
        label.push_str(if self.key == " " { "Space" } else { &self.key });
        label
    }
    pub fn to_event(&self) -> Option<KeyEvent> {
        if self.physical {
            return None;
        }
        let code = match self.key.as_str() {
            "Esc" => KeyCode::Esc,
            "Enter" => KeyCode::Enter,
            "Tab" if self.modifiers & KeyModifiers::SHIFT.bits() != 0 => KeyCode::BackTab,
            "Tab" => KeyCode::Tab,
            "Backspace" => KeyCode::Backspace,
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            s if s.starts_with('F') && s.len() > 1 => KeyCode::F(s[1..].parse().ok()?),
            s if s.chars().count() == 1 => KeyCode::Char(s.chars().next()?),
            _ => return None,
        };
        Some(KeyEvent::new(
            code,
            KeyModifiers::from_bits_truncate(self.modifiers),
        ))
    }
}
/// Whitespace separates named keys. Compact Vim expressions (`ciw`, `gg`) split into chars.
pub fn sequence(input: &str, leader: &str, local: &str) -> Result<Vec<Stroke>, String> {
    let expanded = input
        .replace("<Leader>", &format!("{leader} "))
        .replace("<LocalLeader>", &format!("{local} "));
    let mut out = vec![];
    for token in expanded.split_whitespace() {
        if token.contains('+')
            || token.starts_with('<')
            || token.starts_with("Physical:")
            || [
                "Space",
                "Enter",
                "Esc",
                "Tab",
                "Backspace",
                "Up",
                "Down",
                "Left",
                "Right",
                "PageUp",
                "PageDown",
                "Home",
                "End",
            ]
            .contains(&token)
            || token.starts_with('F') && token[1..].parse::<u8>().is_ok()
        {
            out.push(Stroke::parse(token)?);
        } else {
            for ch in token.chars() {
                out.push(Stroke::parse(&ch.to_string())?);
            }
        }
    }
    if out.is_empty() {
        return Err("empty sequence".into());
    }
    Ok(out)
}
