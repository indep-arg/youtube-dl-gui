use crate::logging::redact::hide_addresses;
use crate::models::payloads::{AppendLogPayload, LogSnapshot};
use indexmap::IndexMap;
use std::collections::{HashSet, VecDeque};
use tauri::{AppHandle, Emitter};

#[derive(Debug)]
struct LogLine {
  text: String,
  repeats: u32,
}

impl LogLine {
  /// A line that came several times in a row is shown once, with a count.
  fn display(&self) -> String {
    if self.repeats > 1 {
      format!("{} (x{})", self.text, self.repeats)
    } else {
      self.text.clone()
    }
  }
}

#[derive(Debug, Default)]
pub struct GroupLog {
  lines: VecDeque<LogLine>,
  /// Index of the first line kept, so indexes stay the same when old lines go.
  start: usize,
  bytes: usize,
}

impl GroupLog {
  fn last_index(&self) -> usize {
    self.start + self.lines.len() - 1
  }
}

#[derive(Debug)]
pub struct LogStore {
  groups: IndexMap<String, GroupLog>,
  total_bytes: usize,
  max_bytes: usize,
  subscribed: HashSet<String>,
}

impl LogStore {
  pub fn new(max_bytes: usize) -> Self {
    Self {
      groups: IndexMap::new(),
      total_bytes: 0,
      max_bytes,
      subscribed: HashSet::new(),
    }
  }

  pub fn subscribe(&mut self, group_id: String) {
    self.subscribed.insert(group_id);
  }

  pub fn unsubscribe(&mut self, group_id: &str) {
    self.subscribed.remove(group_id);
  }

  pub fn append_lines<'a, I>(&mut self, app: &AppHandle, group: &str, lines: I)
  where
    I: IntoIterator<Item = &'a str>,
  {
    for line in lines {
      self.append_line(app, group, line);
    }
  }

  pub fn append_line(&mut self, app: &AppHandle, group_id: &str, line: &str) {
    if let Some(payload) = self.push(group_id, line)
      && let Err(err) = app.emit("logging_append", &payload)
    {
      tracing::debug!("Failed to emit log event: {err}");
    }
  }

  /// Stores a line and returns what to send to a subscribed viewer.
  fn push(&mut self, group_id: &str, line: &str) -> Option<AppendLogPayload> {
    let line = hide_addresses(line);
    let line = line.as_ref();

    let entry = self.groups.entry(group_id.to_string()).or_default();
    match entry.lines.back_mut() {
      Some(last) if last.text == line => last.repeats += 1,
      _ => {
        entry.lines.push_back(LogLine {
          text: line.to_owned(),
          repeats: 1,
        });
        entry.bytes += line.len();
        self.total_bytes += line.len();
      }
    }

    let payload = self
      .subscribed
      .contains(group_id)
      .then(|| AppendLogPayload {
        group_id: group_id.to_string(),
        index: entry.last_index(),
        line: entry.lines.back().map(LogLine::display).unwrap_or_default(),
      });

    self.evict_if_needed(group_id);
    payload
  }

  pub fn snapshot(&self, group_id: &str) -> LogSnapshot {
    self
      .groups
      .get(group_id)
      .map(|group| LogSnapshot {
        start: group.start,
        lines: group.lines.iter().map(LogLine::display).collect(),
      })
      .unwrap_or_default()
  }

  pub fn remove_group(&mut self, group_id: &String) {
    if let Some(old) = self.groups.shift_remove(group_id) {
      self.total_bytes = self.total_bytes.saturating_sub(old.bytes);
    }
    self.subscribed.remove(group_id);
  }

  /// Drops the oldest logs of other groups first. The group being written is
  /// never dropped whole: its oldest lines go instead, and its last line stays.
  fn evict_if_needed(&mut self, writing: &str) {
    while self.total_bytes > self.max_bytes {
      if let Some(index) = self.groups.keys().position(|id| id != writing) {
        if let Some((old_id, old_log)) = self.groups.shift_remove_index(index) {
          self.total_bytes = self.total_bytes.saturating_sub(old_log.bytes);
          self.subscribed.remove(&old_id);
        }
        continue;
      }

      let Some(group) = self.groups.get_mut(writing) else {
        break;
      };
      if group.lines.len() <= 1 {
        break;
      }
      if let Some(old) = group.lines.pop_front() {
        group.start += 1;
        group.bytes = group.bytes.saturating_sub(old.text.len());
        self.total_bytes = self.total_bytes.saturating_sub(old.text.len());
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn store(max_bytes: usize) -> LogStore {
    let mut store = LogStore::new(max_bytes);
    store.subscribe("g".into());
    store
  }

  #[test]
  fn repeated_lines_are_counted_on_one_line() {
    let mut store = store(1 << 20);
    let first = store.push("g", "[download] Retrying").unwrap();
    let second = store.push("g", "[download] Retrying").unwrap();
    let third = store.push("g", "[download] Retrying").unwrap();
    let other = store.push("g", "[download] Done").unwrap();

    assert_eq!(
      (first.index, first.line.as_str()),
      (0, "[download] Retrying")
    );
    assert_eq!(
      (second.index, second.line.as_str()),
      (0, "[download] Retrying (x2)")
    );
    assert_eq!(
      (third.index, third.line.as_str()),
      (0, "[download] Retrying (x3)")
    );
    assert_eq!(other.index, 1);
    assert_eq!(
      store.snapshot("g").lines,
      ["[download] Retrying (x3)", "[download] Done"]
    );
  }

  #[test]
  fn only_consecutive_lines_are_merged() {
    let mut store = store(1 << 20);
    for line in ["a", "b", "a"] {
      store.push("g", line);
    }
    assert_eq!(store.snapshot("g").lines, ["a", "b", "a"]);
  }

  #[test]
  fn nothing_is_sent_without_a_viewer() {
    let mut store = LogStore::new(1 << 20);
    assert!(store.push("g", "line").is_none());
    assert_eq!(store.snapshot("g").lines, ["line"]);
  }

  #[test]
  fn other_groups_are_dropped_before_the_one_being_written() {
    let mut store = store(10);
    store.push("old", "12345");
    store.push("g", "12345");
    store.push("g", "67890");
    assert!(store.snapshot("old").lines.is_empty());
    assert_eq!(store.snapshot("g").lines, ["12345", "67890"]);
  }

  #[test]
  fn a_group_larger_than_the_limit_keeps_its_latest_lines() {
    let mut store = store(10);
    for line in ["aaaa", "bbbb", "cccc", "dddd"] {
      store.push("g", line);
    }
    let snapshot = store.snapshot("g");
    assert_eq!(snapshot.start, 2);
    assert_eq!(snapshot.lines, ["cccc", "dddd"]);
    // The viewer stays subscribed and indexes keep counting.
    assert_eq!(store.push("g", "eeee").unwrap().index, 4);
  }
}
