use crate::SharedLibrary;
use crate::state::library::QueuedLink;
use serde_json::json;
use tauri::{AppHandle, State};

/// Replaces the saved queue with the links that are in the queue now.
#[tauri::command]
pub fn queue_save(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  queue: Vec<QueuedLink>,
) -> Result<(), String> {
  library
    .apply_patch(&json!({ "queue": queue }), &app)
    .map(|_| ())
    .map_err(|e| e.to_string())
}
