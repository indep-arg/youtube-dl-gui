use crate::SharedLibrary;
use crate::state::library::{HistoryEntry, MAX_HISTORY, add_to_history, remove_from_history};
use serde_json::{Value, json};
use tauri::{AppHandle, State};

/// Saves a change of the history away from the main thread, which draws the
/// window: the whole file is written again each time.
async fn update_history(
  app: AppHandle,
  library: &SharedLibrary,
  change: impl FnOnce(&[HistoryEntry]) -> Vec<HistoryEntry> + Send + 'static,
) -> Result<Vec<HistoryEntry>, String> {
  let library = library.clone();
  tauri::async_runtime::spawn_blocking(move || {
    library
      .update(&app, |current| -> Value {
        json!({ "history": change(&current.history) })
      })
      .map(|library| library.history)
      .map_err(|e| e.to_string())
  })
  .await
  .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn history_add(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  entry: HistoryEntry,
) -> Result<Vec<HistoryEntry>, String> {
  update_history(app, &library, move |history| {
    add_to_history(history, entry, MAX_HISTORY)
  })
  .await
}

#[tauri::command]
pub async fn history_remove(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  id: String,
) -> Result<Vec<HistoryEntry>, String> {
  update_history(app, &library, move |history| {
    remove_from_history(history, &id)
  })
  .await
}

#[tauri::command]
pub async fn history_clear(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
) -> Result<Vec<HistoryEntry>, String> {
  update_history(app, &library, |_| Vec::new()).await
}
