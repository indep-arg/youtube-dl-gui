use std::collections::HashMap;

use tauri::State;

use crate::SharedConfig;
use crate::diagnostics::{Report, check_tools, render, system_name};
use crate::paths::PathsManager;

/// A summary of the app, the system and the tools, to paste into a bug report.
#[tauri::command]
pub async fn app_diagnostics(
  app: tauri::AppHandle,
  config: State<'_, SharedConfig>,
  paths: State<'_, PathsManager>,
) -> Result<String, String> {
  let config = config.load();
  let bin_dir = paths.bin_dir().clone();

  let installed = tokio::fs::read(bin_dir.join("metadata.json"))
    .await
    .ok()
    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    .and_then(|meta| {
      serde_json::from_value::<HashMap<String, String>>(meta["versions"].clone()).ok()
    })
    .unwrap_or_default();

  let install = if paths.is_portable_app() {
    "portable"
  } else if std::env::var_os("APPIMAGE").is_some() {
    "AppImage"
  } else if paths.is_snap_app() {
    "Snap"
  } else if paths.is_microsoft_store_app() {
    "Microsoft Store"
  } else {
    "installed"
  };

  let report = Report {
    app_version: app.package_info().version.to_string(),
    system: system_name(),
    install: install.to_string(),
    webview: tauri::webview_version().unwrap_or_else(|_| "unknown".into()),
    language: config.appearance.language.clone(),
    theme: config.appearance.theme.clone(),
    update_tools: config.update.update_binaries,
    update_app: config.update.update_app,
    yt_dlp_channel: config.update.yt_dlp_channel.as_str().to_string(),
    tools: check_tools(&bin_dir, &installed).await,
  };
  Ok(render(&report))
}
