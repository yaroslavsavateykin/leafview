//! Terminal-stream graphics negotiation: works across SSH without remote files.
use anyhow::{Context, Result};
use ratatui_image::picker::{cap_parser::QueryStdioOptions, Picker, ProtocolType};
use std::time::Duration;

pub fn query_timeout(remote: bool, override_ms: Option<&str>) -> Result<Duration> {
    let millis = match override_ms {
        Some(value) => value
            .parse::<u64>()
            .context("LEAFVIEW_QUERY_TIMEOUT_MS must be an integer")?,
        None => {
            if remote {
                3000
            } else {
                1000
            }
        }
    };
    anyhow::ensure!(
        (100..=15000).contains(&millis),
        "LEAFVIEW_QUERY_TIMEOUT_MS must be 100..15000"
    );
    Ok(Duration::from_millis(millis))
}

pub fn picker() -> Result<Picker> {
    let mode = std::env::var("LEAFVIEW_GRAPHICS")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(mode.as_str(), "safe" | "halfblocks") {
        return Ok(Picker::halfblocks());
    }
    anyhow::ensure!(
        matches!(mode.as_str(), "" | "auto" | "native" | "kitty"),
        "unknown LEAFVIEW_GRAPHICS mode: {mode}"
    );
    let remote =
        std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some();
    let timeout = query_timeout(
        remote,
        std::env::var("LEAFVIEW_QUERY_TIMEOUT_MS").ok().as_deref(),
    )?;
    let mut picker = query_picker(timeout).unwrap_or_else(|_| Picker::halfblocks());
    // Explicit opt-in only: SSH environment variables do not prove that the
    // client implements Unicode Kitty placements. Never force it automatically.
    if mode == "kitty" {
        picker.set_protocol_type(ProtocolType::Kitty);
    }
    Ok(picker)
}

// Upstream's timeout returns while its stdin reader thread stays alive. That
// thread can steal later navigation keys. Poll the terminal synchronously so
// there is exactly one input owner, even when a terminal never answers.
#[cfg(target_os = "linux")]
fn query_picker(timeout: Duration) -> Result<Picker> {
    use ratatui_image::{
        picker::cap_parser::{Parser, Response},
        FontSize,
    };
    use std::io::Write;
    let tmux = std::env::var_os("TMUX").is_some()
        || std::env::var("TERM").is_ok_and(|term| term.starts_with("tmux"));
    let mut options = QueryStdioOptions::default();
    if ["WEZTERM_EXECUTABLE", "KONSOLE_VERSION"]
        .iter()
        .any(|key| std::env::var(key).is_ok_and(|value| !value.is_empty()))
    {
        options.blacklist_protocols = vec![ProtocolType::Kitty, ProtocolType::Sixel];
    }
    let query = Parser::query(tmux, options);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(query.as_bytes())?;
    stdout.flush()?;
    drop(stdout);
    let deadline = std::time::Instant::now() + timeout;
    let mut parser = Parser::new();
    let mut protocol = None;
    let mut font = None;
    'query: loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let mut fd = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        // Poll/read use a single byte: no detached reader and no read-ahead
        // beyond the terminating status report into subsequent key events.
        let ready = unsafe { libc::poll(&mut fd, 1, remaining.as_millis().max(1) as i32) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if ready == 0 {
            break;
        }
        let mut byte = 0u8;
        if unsafe { libc::read(libc::STDIN_FILENO, (&mut byte as *mut u8).cast(), 1) } != 1 {
            break;
        }
        for response in parser.push(char::from(byte)) {
            match response {
                Response::Kitty => protocol = Some(ProtocolType::Kitty),
                Response::Sixel if protocol.is_none() => protocol = Some(ProtocolType::Sixel),
                Response::CellSize(Some((width, height))) => {
                    font = Some(FontSize::new(width, height))
                }
                Response::Status => break 'query,
                _ => {}
            }
        }
    }
    let font = font.unwrap_or_else(|| {
        crossterm::terminal::window_size()
            .ok()
            .and_then(|size| {
                let width = size.width.checked_div(size.columns)?;
                let height = size.height.checked_div(size.rows)?;
                (width > 0 && height > 0).then_some(FontSize::new(width, height))
            })
            .unwrap_or(FontSize::new(10, 20))
    });
    #[allow(deprecated)]
    let mut picker = Picker::from_fontsize(font);
    if let Some(protocol) = protocol {
        picker.set_protocol_type(protocol);
    }
    Ok(picker)
}

#[cfg(not(target_os = "linux"))]
fn query_picker(timeout: Duration) -> Result<Picker> {
    Ok(Picker::from_query_stdio_with_options(QueryStdioOptions {
        timeout,
        ..Default::default()
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_queries_allow_latency_and_overrides_are_bounded() {
        assert_eq!(query_timeout(true, None).unwrap(), Duration::from_secs(3));
        assert_eq!(query_timeout(false, None).unwrap(), Duration::from_secs(1));
        assert_eq!(
            query_timeout(true, Some("5000")).unwrap(),
            Duration::from_secs(5)
        );
        for value in ["0", "99", "15001", "bad"] {
            assert!(query_timeout(true, Some(value)).is_err());
        }
    }
}
