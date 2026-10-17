use crate::state::library::{add_to_history, remove_from_history, HistoryEntry, MAX_HISTORY};
use crate::SharedLibrary;
use serde_json::json;
use tauri::{AppHandle, State};

fn save_history(
  app: &AppHandle,
  library: &SharedLibrary,
  history: Vec<HistoryEntry>,
) -> Result<Vec<HistoryEntry>, String> {
  library
    .apply_patch(&json!({ "history": history }), app)
    .map(|library| library.history)
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_add(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  entry: HistoryEntry,
) -> Result<Vec<HistoryEntry>, String> {
  let history = add_to_history(&library.load().history, entry, MAX_HISTORY);
  save_history(&app, &library, history)
}

#[tauri::command]
pub fn history_remove(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  id: String,
) -> Result<Vec<HistoryEntry>, String> {
  let history = remove_from_history(&library.load().history, &id);
  save_history(&app, &library, history)
}

#[tauri::command]
pub fn history_clear(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
) -> Result<Vec<HistoryEntry>, String> {
  save_history(&app, &library, Vec::new())
}
