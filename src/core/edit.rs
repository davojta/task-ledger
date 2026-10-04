//! Line-level edits of `task.md` frontmatter. Content is split into lines
//! (endings kept); only frontmatter lines are classified and touched, every
//! other line is emitted unchanged. New lines reuse the file's line ending.
use std::io::Write;
use std::path::Path;

use crate::core::error::{LedgerError, Result};
use crate::core::record::{parse_yaml, split_frontmatter, HistoryEntry};

/// Set top-level frontmatter `key` to `value` in the full `task.md` content.
/// Keeps a trailing `  # comment` on the edited line. Inserts `key: value`
/// before the closing `---` when absent. Fails (Invalid) when the existing
/// value spans multiple lines or the result no longer parses.
pub fn set_scalar(content: &str, key: &str, value: &str) -> Result<String> {
    let mut lines = split_lines(content);
    let close = closing_index(&lines)?;
    let eol = eol_of(&lines[0]).to_string();
    match find_key(&lines[1..close], key).map(|i| i + 1) {
        Some(i) => {
            let continued = lines
                .get(i + 1)
                .filter(|_| i + 1 < close)
                .is_some_and(|l| is_indented(l));
            if continued {
                return Err(multi_line(key));
            }
            lines[i] = replace_value(&lines[i], key, value)?;
        }
        None => lines.insert(close, format!("{key}: {value}{eol}")),
    }
    guard(lines.concat())
}

/// Append one entry to top-level `history` as `  - {at: D, to: S}` (plus
/// `, note: "<json-escaped>"`). Handles block lists, `[]`/empty values and a
/// missing key. Existing entries stay byte-identical. Fails (Invalid) when the
/// result no longer parses.
pub fn append_history(content: &str, entry: &HistoryEntry) -> Result<String> {
    let mut lines = split_lines(content);
    let close = closing_index(&lines)?;
    let eol = eol_of(&lines[0]).to_string();
    let item = format!("  - {}{eol}", entry_flow(entry)?);
    match find_key(&lines[1..close], "history").map(|i| i + 1) {
        Some(i) => {
            let inline = lines[i]["history:".len()..].trim();
            if inline == "[]" {
                lines[i] = format!("history:{}", eol_of(&lines[i]));
                lines.insert(i + 1, item);
            } else if inline.is_empty() || inline.starts_with('#') {
                let last = block_end(&lines[i + 1..close]) + i;
                lines.insert(last + 1, item);
            } else {
                return Err(LedgerError::Invalid(
                    "`history` is not a block list; edit it manually".into(),
                ));
            }
        }
        None => {
            lines.insert(close, item);
            lines.insert(close, format!("history:{eol}"));
        }
    }
    guard(lines.concat())
}

/// Write via a temp file in the same directory, fsync, then rename over `path`.
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    let io = |e| LedgerError::io(path, e);
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(io)?;
    tmp.write_all(content.as_bytes()).map_err(io)?;
    tmp.as_file().sync_all().map_err(io)?;
    tmp.persist(path).map_err(|e| io(e.error))?;
    Ok(())
}

fn split_lines(content: &str) -> Vec<String> {
    content.split_inclusive('\n').map(str::to_string).collect()
}

fn eol_of(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn is_indented(line: &str) -> bool {
    line.starts_with([' ', '\t']) && !line.trim().is_empty()
}

fn closing_index(lines: &[String]) -> Result<usize> {
    let opened = lines.first().map(|l| l.trim_end()) == Some("---");
    opened
        .then(|| lines.iter().skip(1).position(|l| l.trim_end() == "---"))
        .flatten()
        .map(|i| i + 1)
        .ok_or_else(|| LedgerError::Invalid("task.md has no valid `---` frontmatter block".into()))
}

fn is_key_line(line: &str, key: &str) -> bool {
    line.strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '\r', '\n']))
}

fn find_key(frontmatter: &[String], key: &str) -> Option<usize> {
    frontmatter.iter().position(|l| is_key_line(l, key))
}

/// Offset (relative to `after_key`) of the last line belonging to the block
/// that follows the key line: indented, `- ` item, or blank-in-between lines.
fn block_end(after_key: &[String]) -> usize {
    let members: Vec<&String> = after_key
        .iter()
        .take_while(|l| l.trim().is_empty() || is_indented(l) || l.starts_with("- "))
        .collect();
    members
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |p| p + 1)
}

fn multi_line(key: &str) -> LedgerError {
    LedgerError::Invalid(format!(
        "`{key}` is not a single-line value; edit it manually"
    ))
}

/// Byte length of the value at the start of `v`: a quoted scalar, or a plain
/// scalar up to a ` #` comment / end of line. `None` when a quote is unterminated.
fn value_len(v: &str) -> Option<usize> {
    let bytes = v.as_bytes();
    match bytes.first()? {
        b'"' => {
            let mut i = 1;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' => i += 2,
                    b'"' => return Some(i + 1),
                    _ => i += 1,
                }
            }
            None
        }
        b'\'' => {
            let mut i = 1;
            while i < bytes.len() {
                match (bytes[i], bytes.get(i + 1)) {
                    (b'\'', Some(b'\'')) => i += 2,
                    (b'\'', _) => return Some(i + 1),
                    _ => i += 1,
                }
            }
            None
        }
        _ => Some(
            (1..bytes.len())
                .find(|&i| bytes[i] == b'#' && matches!(bytes[i - 1], b' ' | b'\t'))
                .map_or(v.trim_end().len(), |i| v[..i].trim_end().len()),
        ),
    }
}

fn replace_value(line: &str, key: &str, value: &str) -> Result<String> {
    let eol = eol_of(line);
    let after = &line[key.len() + 1..line.len() - eol.len()];
    let current = after.trim_start_matches([' ', '\t']);
    let gap = &after[..after.len() - current.len()];
    if current.is_empty() || current.starts_with(['|', '>', '#']) {
        return Err(multi_line(key));
    }
    let len = value_len(current)
        .ok_or_else(|| LedgerError::Invalid(format!("`{key}` has an unterminated quoted value")))?;
    Ok(format!("{key}:{gap}{value}{}{eol}", &current[len..]))
}

fn entry_flow(entry: &HistoryEntry) -> Result<String> {
    let note = entry
        .note
        .as_ref()
        .map(|n| serde_json::to_string(n).map(|json| format!(", note: {json}")))
        .transpose()
        .map_err(|e| LedgerError::Internal(format!("cannot encode history note: {e}")))?
        .unwrap_or_default();
    Ok(format!("{{at: {}, to: {}{note}}}", entry.at, entry.to))
}

fn guard(result: String) -> Result<String> {
    let invalid = |m: String| LedgerError::Invalid(format!("edit produced invalid task.md: {m}"));
    let (yaml, _) = split_frontmatter(&result).map_err(|p| invalid(p.message))?;
    parse_yaml(&yaml).map_err(|p| invalid(p.message))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::lifecycle::Status;

    const FIXTURE: &str = "---\nschema: 1\nid: ingest/2026-09-12-cdc\n# owned by CLI below\nstatus: active   # current\nowner: me\n\ncreated: 2026-09-12\nupdated: 2026-09-12\nhistory:\n  - {at: 2026-09-12, to: backlog}\n  - {at: 2026-09-20, to: active}\n---\nBody with status: active and --- inside\n";

    fn entry(note: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            at: "2026-10-01".into(),
            to: Status::Review,
            note: note.map(str::to_string),
        }
    }

    fn parsed(content: &str) -> serde_json::Map<String, serde_json::Value> {
        let (yaml, _) = split_frontmatter(content).unwrap();
        parse_yaml(&yaml).unwrap()
    }

    #[test]
    fn set_scalar_keeps_comment_and_other_bytes() {
        let out = set_scalar(FIXTURE, "status", "review").unwrap();
        assert_eq!(
            out,
            FIXTURE.replace("status: active   # current", "status: review   # current")
        );
        assert_eq!(parsed(&out)["status"], "review");
    }

    #[test]
    fn set_scalar_missing_key_inserts_before_closing() {
        let out = set_scalar(FIXTURE, "stage", "design").unwrap();
        assert_eq!(
            out,
            FIXTURE.replace("---\nBody", "stage: design\n---\nBody")
        );
        assert_eq!(parsed(&out)["stage"], "design");
    }

    #[test]
    fn set_scalar_ignores_body_and_prefix_keys() {
        let content = "---\nstatus_changed: x\n---\nstatus: active\n";
        let out = set_scalar(content, "status", "done").unwrap();
        assert_eq!(
            out,
            "---\nstatus_changed: x\nstatus: done\n---\nstatus: active\n"
        );
    }

    #[test]
    fn set_scalar_quoted_value_with_hash() {
        let content = "---\ntitle: \"a # b\"  # c\nnote: 'it''s # x'\n---\n";
        let out = set_scalar(content, "title", "\"new\"").unwrap();
        assert_eq!(out, "---\ntitle: \"new\"  # c\nnote: 'it''s # x'\n---\n");
        let out = set_scalar(content, "note", "'z'").unwrap();
        assert_eq!(out, "---\ntitle: \"a # b\"  # c\nnote: 'z'\n---\n");
    }

    #[test]
    fn set_scalar_plain_value_hash_without_space_is_not_comment() {
        let content = "---\nrepo: a#b\n---\n";
        assert_eq!(
            set_scalar(content, "repo", "c").unwrap(),
            "---\nrepo: c\n---\n"
        );
    }

    #[test]
    fn set_scalar_rejects_multi_line_values() {
        for content in [
            "---\ntitle:\n  long\n---\n",
            "---\ntitle: |\n  long\n---\n",
            "---\ntitle: >-\n  long\n---\n",
            "---\ntitle: a\n  b\n---\n",
        ] {
            let err = set_scalar(content, "title", "x").unwrap_err();
            assert!(
                matches!(&err, LedgerError::Invalid(m) if m == "`title` is not a single-line value; edit it manually"),
                "{content:?}: {err}"
            );
        }
    }

    #[test]
    fn set_scalar_without_frontmatter_is_invalid() {
        assert!(matches!(
            set_scalar("no\n", "a", "b"),
            Err(LedgerError::Invalid(_))
        ));
        assert!(matches!(
            set_scalar("---\na: 1\n", "a", "b"),
            Err(LedgerError::Invalid(_))
        ));
    }

    #[test]
    fn set_scalar_reparse_guard() {
        let err = set_scalar(FIXTURE, "status", "[").unwrap_err();
        assert!(matches!(err, LedgerError::Invalid(_)));
    }

    #[test]
    fn append_history_after_last_item() {
        let out = append_history(FIXTURE, &entry(None)).unwrap();
        assert_eq!(
            out,
            FIXTURE.replace(
                "to: active}\n---",
                "to: active}\n  - {at: 2026-10-01, to: review}\n---"
            )
        );
        assert_eq!(parsed(&out)["history"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn append_history_stops_before_following_key() {
        let content = "---\nhistory:\n  - {at: 2026-09-12, to: backlog}\nowner: me\n---\n";
        let out = append_history(content, &entry(None)).unwrap();
        assert_eq!(
            out,
            "---\nhistory:\n  - {at: 2026-09-12, to: backlog}\n  - {at: 2026-10-01, to: review}\nowner: me\n---\n"
        );
    }

    #[test]
    fn append_history_note_is_json_quoted() {
        let out = append_history(FIXTURE, &entry(Some("say \"hi\": ok"))).unwrap();
        assert!(
            out.contains("  - {at: 2026-10-01, to: review, note: \"say \\\"hi\\\": ok\"}\n---\n")
        );
        let map = parsed(&out);
        assert_eq!(map["history"][2]["note"], "say \"hi\": ok");
    }

    #[test]
    fn append_history_empty_flow_list() {
        let content = "---\nid: a\nhistory: []\nowner: me\n---\nbody\n";
        let out = append_history(content, &entry(None)).unwrap();
        assert_eq!(
            out,
            "---\nid: a\nhistory:\n  - {at: 2026-10-01, to: review}\nowner: me\n---\nbody\n"
        );
        assert_eq!(parsed(&out)["history"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn append_history_absent_key() {
        let content = "---\nid: a\n---\nbody\n";
        let out = append_history(content, &entry(None)).unwrap();
        assert_eq!(
            out,
            "---\nid: a\nhistory:\n  - {at: 2026-10-01, to: review}\n---\nbody\n"
        );
        assert_eq!(parsed(&out)["history"][0]["to"], "review");
    }

    #[test]
    fn crlf_preserved_on_untouched_and_new_lines() {
        let crlf = FIXTURE.replace('\n', "\r\n");
        let out = set_scalar(&crlf, "status", "review").unwrap();
        assert_eq!(
            out,
            crlf.replace("status: active   # current", "status: review   # current")
        );
        let out = append_history(&crlf, &entry(None)).unwrap();
        assert!(out.contains("to: active}\r\n  - {at: 2026-10-01, to: review}\r\n---\r\n"));
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
        let out = set_scalar(&crlf, "stage", "x").unwrap();
        assert!(out.contains("stage: x\r\n---\r\n"));
    }

    #[test]
    fn write_atomic_writes_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("task.md");
        write_atomic(&path, "one\n").unwrap();
        write_atomic(&path, "two\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two\n");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["task.md"]);
    }

    #[test]
    fn write_atomic_missing_dir_is_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = write_atomic(&dir.path().join("nope/task.md"), "x").unwrap_err();
        assert!(matches!(err, LedgerError::Io { .. }));
    }
}
