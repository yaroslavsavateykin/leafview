//! Terminal clipboard: OSC 52 works over SSH without spawning desktop helpers.
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Write;

fn sequence(text: &str, tmux: bool) -> Result<String> {
    // Keep both allocations and terminal control traffic bounded.
    ensure!(
        text.len() <= 1024 * 1024,
        "diagnostic exceeds 1 MiB clipboard limit"
    );
    let osc = format!("\x1b]52;c;{}\x07", STANDARD.encode(text.as_bytes()));
    Ok(if tmux {
        format!("\x1bPtmux;{}\x1b\\", osc.replace('\x1b', "\x1b\x1b"))
    } else {
        osc
    })
}
pub fn copy(text: &str) -> Result<()> {
    let sequence = sequence(text, std::env::var_os("TMUX").is_some())?;
    let mut out = std::io::stdout().lock();
    out.write_all(sequence.as_bytes())?;
    out.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_encodes_controls_and_unicode() {
        let text = "Ошибка\n\x1b]bad";
        assert_eq!(
            sequence(text, false).unwrap(),
            format!("\x1b]52;c;{}\x07", STANDARD.encode(text))
        );
        let wrapped = sequence(text, true).unwrap();
        assert!(wrapped.starts_with("\x1bPtmux;\x1b\x1b]52;c;"));
        assert!(wrapped.ends_with("\x07\x1b\\"));
        assert!(sequence(&"a".repeat(1024 * 1024 + 1), false).is_err());
    }
}
