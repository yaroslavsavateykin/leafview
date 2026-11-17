//! Metadata-only tmux lifecycle tracking. No configuration or hooks are changed.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    pub visible: bool,
    identity: String,
}

impl State {
    pub const FORMAT: &'static str = "#{window_active}|#{window_zoomed_flag}|#{pane_active}|#{pane_in_mode}|#{client_pid}|#{client_created}|#{client_tty}|#{session_attached}|#{window_layout}";

    pub fn parse(value: &str) -> Option<Self> {
        let fields: Vec<_> = value.trim().split('|').collect();
        if fields.len() != 9 || fields[..4].iter().any(|v| !matches!(*v, "0" | "1")) {
            return None;
        }
        Some(Self {
            visible: fields[0] == "1" && (fields[1] == "0" || fields[2] == "1") && fields[3] == "0",
            identity: fields[4..].join("|"),
        })
    }

    pub fn needs_replay(&self, previous: &Self) -> bool {
        self.visible && (!previous.visible || self.identity != previous.identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_and_client_changes_trigger_recovery() {
        let state = |s| State::parse(s).unwrap();
        let visible = state("1|0|0|0|12|100|/dev/pts/2|1|layout");
        assert!(!visible.needs_replay(&visible));
        for hidden in [
            "0|0|0|0|12|100|/dev/pts/2|1|layout",
            "1|1|0|0|12|100|/dev/pts/2|1|layout",
            "1|0|0|1|12|100|/dev/pts/2|1|layout",
        ] {
            let hidden = state(hidden);
            assert!(!hidden.visible);
            assert!(visible.needs_replay(&hidden));
            assert!(!hidden.needs_replay(&visible));
        }
        let reattached = state("1|0|0|0|14|200|/dev/pts/3|1|layout");
        assert!(reattached.needs_replay(&visible));
        assert!(state("1|1|1|0|12|100|/dev/pts/2|1|layout").visible);
    }

    #[test]
    fn invalid_metadata_is_not_a_visibility_transition() {
        for value in ["", "1", "error", "1|0|0|?|12|100|tty|1|layout"] {
            assert!(State::parse(value).is_none());
        }
    }
}
