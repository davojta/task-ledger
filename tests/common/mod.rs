#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

pub const TODAY: &str = "2026-10-04";

/// A temp ledger root with `ledger.toml`; commands run with `LEDGER_ROOT`
/// pointing at it and `LEDGER_TODAY` fixed to `TODAY`.
pub struct Fixture {
    dir: TempDir,
}

impl Fixture {
    pub fn new() -> Fixture {
        let dir = TempDir::new().expect("create temp dir");
        fs::write(dir.path().join("ledger.toml"), "").expect("write ledger.toml");
        Fixture { dir }
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    /// Write `content` to `<root>/<rel>`, creating parent dirs.
    pub fn write(&self, rel: &str, content: &str) -> &Fixture {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
        fs::write(path, content).expect("write file");
        self
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).expect("read file")
    }

    /// Write `<id>/task.md` with a minimal valid record for `id` and `status`.
    pub fn task(&self, id: &str, status: &str) -> &Fixture {
        self.write(&format!("{id}/task.md"), &task_md(id, status))
    }

    pub fn cmd(&self) -> Command {
        self.bin("ledger")
    }

    pub fn bin(&self, name: &str) -> Command {
        let mut cmd = Command::cargo_bin(name).expect("binary built");
        cmd.env("LEDGER_ROOT", self.root())
            .env("LEDGER_TODAY", TODAY)
            .current_dir(self.root());
        cmd
    }
}

/// Minimal valid `task.md` for `id` (`<project>/<slug>`).
pub fn task_md(id: &str, status: &str) -> String {
    let project = id.split('/').next().expect("project");
    format!(
        "---\nschema: 1\nid: {id}\ntitle: Task {id}\nproject: {project}\nstatus: {status}\ncreated: 2026-09-01\nupdated: 2026-09-01\n---\n"
    )
}
