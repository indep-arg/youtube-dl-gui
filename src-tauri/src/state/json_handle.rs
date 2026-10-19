use crate::paths::PathsManager;
use crate::state::json_state::{JsonBackedState, json_merge};
use arc_swap::ArcSwap;
use serde_json::{Map, Value};
use std::{
  error::Error,
  sync::{Arc, Mutex},
};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};

pub struct JsonStoreHandle<T: JsonBackedState> {
  swap: Arc<ArcSwap<T>>,
  store: Arc<Store<Wry>>,
  /// Changes read the current value and write a new one: one at a time, or
  /// two at once would lose one of them.
  writing: Mutex<()>,
}

impl<T: JsonBackedState> JsonStoreHandle<T> {
  pub fn init(app: &AppHandle<Wry>) -> Result<Self, Box<dyn Error>> {
    let paths_manager = app.state::<PathsManager>();
    let data_root = paths_manager.app_dir();
    let store_path = data_root.join(T::STORE_FILE);
    let store = app.store(store_path)?;

    let raw = store
      .get(T::ROOT_KEY)
      .unwrap_or_else(|| Value::Object(Map::new()));

    let mut initial = T::materialize(&raw)?;
    T::before_initialized(app, &mut initial);

    let to_save = serde_json::to_value(&initial)?;
    store.set(T::ROOT_KEY, to_save);
    store.save()?;

    Ok(Self {
      swap: Arc::new(ArcSwap::from_pointee(initial)),
      store,
      writing: Mutex::new(()),
    })
  }

  #[inline]
  pub fn load(&self) -> Arc<T> {
    self.swap.load_full()
  }

  pub fn apply_patch(&self, patch: &Value, app: &AppHandle<Wry>) -> Result<T, Box<dyn Error>> {
    let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
    self.write_patch(patch, app)
  }

  /// Applies the patch `make_patch` returns for the current value, with no
  /// other change in between.
  pub fn update(
    &self,
    app: &AppHandle<Wry>,
    make_patch: impl FnOnce(&T) -> Value,
  ) -> Result<T, Box<dyn Error>> {
    let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
    let patch = make_patch(self.swap.load_full().as_ref());
    self.write_patch(&patch, app)
  }

  fn write_patch(&self, patch: &Value, app: &AppHandle<Wry>) -> Result<T, Box<dyn Error>> {
    let old_value = self.swap.load_full();
    let mut raw = serde_json::to_value(old_value.as_ref())?;

    json_merge(&mut raw, patch);

    let new_value = T::materialize(&raw)?;

    let to_store = serde_json::to_value(&new_value)?;
    self.store.set(T::ROOT_KEY, to_store);
    self.store.save()?;
    self.swap.store(Arc::new(new_value.clone()));
    // After the new value is stored: the reactions read it from the app state.
    T::on_updated(app, &old_value, &new_value);

    Ok(new_value)
  }

  pub fn reset(&self, app: &AppHandle<Wry>) -> Result<T, Box<dyn Error>> {
    let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
    let old_value = self.swap.load_full();
    let default_value = T::default_value();
    let raw = serde_json::to_value(&default_value)?;

    self.store.set(T::ROOT_KEY, raw);
    self.store.save()?;
    self.swap.store(Arc::new(default_value.clone()));
    T::on_updated(app, &old_value, &default_value);

    Ok(default_value)
  }
}
