use crate::SharedConfig;
use serde_json::json;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

/// Asks where to save the settings and writes them there as JSON.
/// Returns `false` when the dialog is cancelled.
#[tauri::command]
pub async fn config_export(app: AppHandle, cfg: State<'_, SharedConfig>) -> Result<bool, String> {
  let (tx, rx) = oneshot::channel();
  app
    .dialog()
    .file()
    .set_file_name("open-video-downloader-settings.json")
    .add_filter("JSON", &["json"])
    .save_file(move |path| {
      let _ = tx.send(path);
    });

  let Some(path) = rx.await.map_err(|e| e.to_string())? else {
    return Ok(false);
  };
  let path = path.into_path().map_err(|e| e.to_string())?;

  let export = json!({
    "app": "Open Video Downloader",
    "version": env!("CARGO_PKG_VERSION"),
    "config": *cfg.load(),
  });
  let contents = serde_json::to_string_pretty(&export).map_err(|e| e.to_string())?;
  std::fs::write(&path, contents).map_err(|e| e.to_string())?;
  Ok(true)
}
