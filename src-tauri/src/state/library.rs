use crate::state::json_handle::JsonStoreHandle;
use crate::state::json_state::JsonBackedState;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How many finished downloads the history keeps; older ones are dropped.
pub const MAX_HISTORY: usize = 1000;

/// What the app remembers between sessions: finished downloads and the links
/// that were still in the queue when it was closed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Library {
  pub history: Vec<HistoryEntry>,
  pub queue: Vec<QueuedLink>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryEntry {
  pub id: String,
  pub url: String,
  pub title: Option<String>,
  pub thumbnail: Option<String>,
  pub uploader: Option<String>,
  /// Where the (main) file was saved, when it is known.
  pub path: Option<String>,
  /// How many files the download produced (more than one for a playlist).
  pub items: u32,
  /// Unix time in milliseconds.
  pub finished_at: u64,
  pub is_live: bool,
}

/// A link that was in the queue. The overrides and options are kept as the
/// frontend sent them, so they come back exactly the same.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QueuedLink {
  pub url: String,
  pub overrides: Option<Value>,
  pub options: Option<Value>,
}

impl JsonBackedState for Library {
  const STORE_FILE: &'static str = "library.store.json";
  const ROOT_KEY: &'static str = "library";

  fn default_value() -> Self {
    Self::default()
  }
}

pub type LibraryHandle = JsonStoreHandle<Library>;

/// Puts `entry` first, replacing an older entry with the same id, and keeps at
/// most `max` entries.
pub fn add_to_history(
  history: &[HistoryEntry],
  entry: HistoryEntry,
  max: usize,
) -> Vec<HistoryEntry> {
  let mut next = Vec::with_capacity(history.len() + 1);
  let id = entry.id.clone();
  next.push(entry);
  next.extend(history.iter().filter(|e| e.id != id).cloned());
  next.truncate(max);
  next
}

pub fn remove_from_history(history: &[HistoryEntry], id: &str) -> Vec<HistoryEntry> {
  history.iter().filter(|e| e.id != id).cloned().collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(id: &str) -> HistoryEntry {
    HistoryEntry {
      id: id.into(),
      url: format!("https://example.com/{id}"),
      ..HistoryEntry::default()
    }
  }

  fn ids(history: &[HistoryEntry]) -> Vec<&str> {
    history.iter().map(|e| e.id.as_str()).collect()
  }

  #[test]
  fn new_entries_go_first() {
    let history = add_to_history(&[entry("a")], entry("b"), 10);
    assert_eq!(ids(&history), vec!["b", "a"]);
  }

  #[test]
  fn an_entry_with_the_same_id_is_replaced_and_moved_first() {
    let history = add_to_history(&[entry("a"), entry("b")], entry("b"), 10);
    assert_eq!(ids(&history), vec!["b", "a"]);
  }

  #[test]
  fn the_history_is_capped() {
    let full: Vec<HistoryEntry> = (0..5).map(|i| entry(&i.to_string())).collect();
    let history = add_to_history(&full, entry("new"), 5);
    assert_eq!(history.len(), 5);
    assert_eq!(history[0].id, "new");
    assert_eq!(history[4].id, "3");
  }

  #[test]
  fn removing_an_entry_keeps_the_others() {
    let history = remove_from_history(&[entry("a"), entry("b"), entry("c")], "b");
    assert_eq!(ids(&history), vec!["a", "c"]);
  }

  #[test]
  fn an_old_library_file_without_a_queue_still_loads() {
    let raw = serde_json::json!({ "history": [{ "id": "a", "url": "https://example.com" }] });
    let library = Library::materialize(&raw).unwrap();
    assert_eq!(library.history.len(), 1);
    assert!(library.queue.is_empty());
  }
}
