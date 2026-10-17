use crate::state::config_models::Config;
use crate::SharedConfig;
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

/// Asks for a settings file exported by the app and applies it on top of the
/// current settings. Returns `None` when the dialog is cancelled.
#[tauri::command]
pub async fn config_import(
  app: AppHandle,
  cfg: State<'_, SharedConfig>,
) -> Result<Option<Config>, String> {
  let (tx, rx) = oneshot::channel();
  app
    .dialog()
    .file()
    .add_filter("JSON", &["json"])
    .pick_file(move |path| {
      let _ = tx.send(path);
    });

  let Some(path) = rx.await.map_err(|e| e.to_string())? else {
    return Ok(None);
  };
  let path = path.into_path().map_err(|e| e.to_string())?;

  let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
  let patch = settings_from_export(&contents)?;
  cfg
    .apply_patch(&patch, &app)
    .map(Some)
    .map_err(|e| e.to_string())
}

/// Accepts the file written by `config_export` or a bare settings object.
fn settings_from_export(contents: &str) -> Result<Value, String> {
  let value: Value =
    serde_json::from_str(contents).map_err(|_| "not a valid JSON file".to_string())?;
  let settings = match value.get("config") {
    Some(config) => config.clone(),
    None => value,
  };
  if !settings.is_object() {
    return Err("the file does not contain settings".into());
  }
  Ok(settings)
}

#[cfg(test)]
mod tests {
  use super::settings_from_export;

  #[test]
  fn reads_an_exported_file() {
    let settings = settings_from_export(
      r#"{"app":"Open Video Downloader","config":{"performance":{"retries":3}}}"#,
    )
    .unwrap();
    assert_eq!(settings["performance"]["retries"], 3);
  }

  #[test]
  fn reads_a_bare_settings_object() {
    let settings = settings_from_export(r#"{"output":{"splitChapters":true}}"#).unwrap();
    assert_eq!(settings["output"]["splitChapters"], true);
  }

  #[test]
  fn rejects_files_without_settings() {
    assert!(settings_from_export("[1, 2]").is_err());
    assert!(settings_from_export(r#"{"config": 5}"#).is_err());
    assert!(settings_from_export("not json").is_err());
  }
}
