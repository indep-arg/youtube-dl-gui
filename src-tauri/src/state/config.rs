use crate::commands::{register_shortcuts, unregister_shortcuts};
use crate::i18n::I18nManager;
use crate::state::config_models::Config;
use crate::state::json_handle::JsonStoreHandle;
use crate::state::json_state::JsonBackedState;
use crate::tray::{create_tray, destroy_tray};
use crate::{DownloadLimiter, FetchLimiter};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

impl JsonBackedState for Config {
  const STORE_FILE: &'static str = "config.store.json";
  const ROOT_KEY: &'static str = "config";

  fn default_value() -> Self {
    Config::default()
  }

  fn before_initialized(app: &AppHandle<Wry>, value: &mut Self) {
    if value.network.enable_proxy.is_none() {
      value.network.enable_proxy =
        Some(value.network.proxy.as_ref().is_some_and(|v| !v.is_empty()));
    }
    if value.output.download_dir.is_none() {
      let download_path = app
        .path()
        .download_dir()
        .unwrap_or_else(|_| std::env::current_dir().expect("couldn’t get current dir"));
      value.output.download_dir = Some(download_path.to_string_lossy().into_owned());
    }
  }

  fn on_updated(app: &AppHandle<Wry>, old_value: &Self, new_value: &Self) {
    let changes = ConfigChanges::between(old_value, new_value);

    if let Some(max) = changes.max_concurrency {
      for limiter in [
        app.try_state::<DownloadLimiter>().map(|l| l.0.clone()),
        app.try_state::<FetchLimiter>().map(|l| l.0.clone()),
      ]
      .into_iter()
      .flatten()
      {
        tauri::async_runtime::spawn(async move {
          limiter.resize(max).await;
        });
      }
    }

    match changes.global_shortcuts {
      Some(true) => register_shortcuts(app),
      Some(false) => unregister_shortcuts(app),
      None => {}
    }

    if let Some(language) = &changes.language {
      let i18n_handle = app.state::<I18nManager>();
      if language == "system" {
        i18n_handle.unset_locale();
      } else {
        i18n_handle.set_locale(language);
      }
    }

    // The tray is built again for a new language, after the locale is set.
    if changes.rebuild_tray {
      destroy_tray(app);
      create_tray(app);
    }

    match changes.auto_start {
      Some(true) => {
        let _ = app.autolaunch().enable();
      }
      Some(false) => {
        let _ = app.autolaunch().disable();
      }
      None => {}
    }
  }
}

/// What a change of the settings has to update. Settings are saved as a
/// whole, so most saves change none of these.
#[derive(Debug, Default, PartialEq)]
struct ConfigChanges {
  max_concurrency: Option<usize>,
  global_shortcuts: Option<bool>,
  language: Option<String>,
  rebuild_tray: bool,
  auto_start: Option<bool>,
}

/// The new value, if it differs from the old one.
fn changed<T: PartialEq>(old: T, new: T) -> Option<T> {
  (old != new).then_some(new)
}

impl ConfigChanges {
  fn between(old: &Config, new: &Config) -> Self {
    let language =
      (old.appearance.language != new.appearance.language).then(|| new.appearance.language.clone());
    Self {
      max_concurrency: changed(
        old.performance.max_concurrency,
        new.performance.max_concurrency,
      ),
      global_shortcuts: changed(old.input.global_shortcuts, new.input.global_shortcuts),
      rebuild_tray: old.system.tray_enabled != new.system.tray_enabled
        || (language.is_some() && new.system.tray_enabled),
      language,
      auto_start: changed(old.system.auto_start_enabled, new.system.auto_start_enabled),
    }
  }
}

pub type ConfigHandle = JsonStoreHandle<Config>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn saving_unchanged_settings_changes_nothing() {
    let config = Config::default();
    assert_eq!(
      ConfigChanges::between(&config, &config.clone()),
      ConfigChanges::default()
    );
  }

  #[test]
  fn each_setting_triggers_only_its_own_update() {
    let old = Config::default();
    let mut new = old.clone();
    new.input.global_shortcuts = !old.input.global_shortcuts;
    new.system.auto_start_enabled = !old.system.auto_start_enabled;
    let changes = ConfigChanges::between(&old, &new);
    assert_eq!(changes.global_shortcuts, Some(new.input.global_shortcuts));
    assert_eq!(changes.auto_start, Some(new.system.auto_start_enabled));
    assert!(!changes.rebuild_tray);
    assert_eq!(changes.max_concurrency, None);
  }

  #[test]
  fn a_new_language_rebuilds_an_enabled_tray() {
    let mut old = Config::default();
    old.system.tray_enabled = true;
    let mut new = old.clone();
    new.appearance.language = "es".into();
    let changes = ConfigChanges::between(&old, &new);
    assert_eq!(changes.language.as_deref(), Some("es"));
    assert!(changes.rebuild_tray);

    old.system.tray_enabled = false;
    new.system.tray_enabled = false;
    assert!(!ConfigChanges::between(&old, &new).rebuild_tray);
  }
}
