use crate::SharedLibrary;
use crate::state::library::QueuedLink;
use serde_json::json;
use tauri::{AppHandle, State};

/// Replaces the saved queue with the links that are in the queue now.
#[tauri::command]
pub async fn queue_save(
  app: AppHandle,
  library: State<'_, SharedLibrary>,
  queue: Vec<QueuedLink>,
) -> Result<(), String> {
  // Written away from the main thread, which draws the window.
  let library = library.inner().clone();
  tauri::async_runtime::spawn_blocking(move || {
    library
      .apply_patch(&json!({ "queue": queue }), &app)
      .map(|_| ())
      .map_err(|e| e.to_string())
  })
  .await
  .map_err(|e| e.to_string())?
}
