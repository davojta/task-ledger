use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::{LedgerError, Result};

/// Declared in lifecycle order; `Ord` follows it.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ValueEnum,
    JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idea,
    Backlog,
    Active,
    Review,
    Done,
    Dropped,
}

impl Status {
    pub const ALL: [Status; 6] = [
        Status::Idea,
        Status::Backlog,
        Status::Active,
        Status::Review,
        Status::Done,
        Status::Dropped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Idea => "idea",
            Status::Backlog => "backlog",
            Status::Active => "active",
            Status::Review => "review",
            Status::Done => "done",
            Status::Dropped => "dropped",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        Status::ALL.into_iter().find(|status| status.as_str() == s)
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Status::Done | Status::Dropped)
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransitionOpts {
    pub reopen: bool,
    pub force: bool,
}

/// Targets reachable from `from` without `--force`.
pub fn allowed_targets(from: Status, reopen: bool) -> Vec<Status> {
    use Status::*;
    match from {
        Idea => vec![Backlog, Dropped],
        Backlog => vec![Active, Dropped],
        Active => vec![Review, Dropped],
        Review => vec![Active, Done, Dropped],
        Done | Dropped if reopen => vec![Backlog],
        Done | Dropped => vec![],
    }
}

/// Ok when `from -> to` is allowed under `opts`; IllegalTransition otherwise
/// (`allowed` = `allowed_targets(from, opts.reopen)` as strings).
/// `from == to` is the caller's no-op case and is not checked here.
pub fn check_transition(from: Status, to: Status, opts: TransitionOpts) -> Result<()> {
    let allowed = allowed_targets(from, opts.reopen);
    if opts.force || allowed.contains(&to) {
        return Ok(());
    }
    let hint = if from.is_terminal() && to == Status::Backlog {
        " (use --reopen)"
    } else {
        ""
    };
    Err(LedgerError::IllegalTransition {
        message: format!("cannot change status from {from} to {to}{hint}"),
        allowed: allowed.iter().map(|s| s.to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Status::*;

    const ALLOWED: [(Status, Status); 9] = [
        (Idea, Backlog),
        (Backlog, Active),
        (Active, Review),
        (Review, Done),
        (Review, Active),
        (Idea, Dropped),
        (Backlog, Dropped),
        (Active, Dropped),
        (Review, Dropped),
    ];

    fn plain() -> TransitionOpts {
        TransitionOpts::default()
    }

    #[test]
    fn parse_round_trips() {
        for s in Status::ALL {
            assert_eq!(Status::parse(s.as_str()), Some(s));
        }
        assert_eq!(Status::parse("wip"), None);
    }

    #[test]
    fn only_listed_pairs_are_allowed_without_flags() {
        for from in Status::ALL {
            for to in Status::ALL.into_iter().filter(|to| *to != from) {
                let expected = ALLOWED.contains(&(from, to));
                assert_eq!(
                    check_transition(from, to, plain()).is_ok(),
                    expected,
                    "{from} -> {to}"
                );
            }
        }
    }

    #[test]
    fn reopen_allows_terminal_to_backlog_only() {
        let opts = TransitionOpts {
            reopen: true,
            force: false,
        };
        assert!(check_transition(Done, Backlog, opts).is_ok());
        assert!(check_transition(Dropped, Backlog, opts).is_ok());
        assert!(check_transition(Done, Active, opts).is_err());
    }

    #[test]
    fn force_allows_anything() {
        let opts = TransitionOpts {
            reopen: false,
            force: true,
        };
        assert!(check_transition(Idea, Done, opts).is_ok());
    }

    #[test]
    fn illegal_transition_names_allowed_targets() {
        let err = check_transition(Idea, Done, plain()).unwrap_err();
        assert_eq!(err.exit_code(), 4);
        assert_eq!(
            err.to_string(),
            "cannot change status from idea to done; allowed: backlog, dropped"
        );
    }

    #[test]
    fn ord_follows_lifecycle() {
        let mut v = vec![Dropped, Idea, Review, Backlog, Done, Active];
        v.sort();
        assert_eq!(v, Status::ALL.to_vec());
    }
}
