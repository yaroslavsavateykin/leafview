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
    let mut picker = Picker::from_query_stdio_with_options(QueryStdioOptions {
        timeout,
        ..Default::default()
    })
    .unwrap_or_else(|_| Picker::halfblocks());
    // Explicit opt-in only: SSH environment variables do not prove that the
    // client implements Unicode Kitty placements. Never force it automatically.
    if mode == "kitty" {
        picker.set_protocol_type(ProtocolType::Kitty);
    }
    Ok(picker)
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
