//! Ledger core: loads, queries and mutates task records under a ledger root.
//!
//! A ledger root is a directory containing `ledger.toml`. Tasks live at
//! `<root>/<project>/<slug>/task.md`; the YAML frontmatter of `task.md` is the
//! single source of truth for a task's metadata. Tasks never move: lifecycle is
//! a `status` field, edited in place by surgical line edits (see `edit`).
//!
//! This module never prints and never exits; every fallible function returns
//! `error::Result`, which the `cli` layer maps to output and exit codes.

pub mod check;
pub mod date;
pub mod edit;
pub mod error;
pub mod lifecycle;
pub mod record;
pub mod resolve;
pub mod root;
pub mod scaffold;
pub mod scan;
