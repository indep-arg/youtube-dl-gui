use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex as StdMutex};
use tokio::sync::watch;

static RUNNING_GROUPS: LazyLock<StdMutex<HashMap<String, watch::Sender<bool>>>> =
  LazyLock::new(|| StdMutex::new(HashMap::new()));

/// Groups whose live recording should be stopped and kept, not thrown away.
static STOP_REQUESTS: LazyLock<StdMutex<HashSet<String>>> =
  LazyLock::new(|| StdMutex::new(HashSet::new()));

/// Stops the downloads of a group like `cancel_group`, but asks live
/// recordings to save what they got.
pub fn stop_group(group_id: &str) {
  STOP_REQUESTS.lock().unwrap().insert(group_id.to_string());
  cancel_group(group_id);
}

pub fn is_stop_requested(group_id: &str) -> bool {
  STOP_REQUESTS.lock().unwrap().contains(group_id)
}

pub fn ensure_group_running(group_id: &str) {
  STOP_REQUESTS.lock().unwrap().remove(group_id);
  let mut map = RUNNING_GROUPS.lock().unwrap();
  if let Some(tx) = map.get(group_id) {
    let _ = tx.send(true);
  } else {
    let (tx, _rx) = watch::channel(true);
    map.insert(group_id.to_string(), tx);
  }
}

pub fn cancel_group(group_id: &str) {
  let mut map = RUNNING_GROUPS.lock().unwrap();
  if let Some(tx) = map.get(group_id) {
    let _ = tx.send(false);
  } else {
    let (tx, _rx) = watch::channel(false);
    map.insert(group_id.to_string(), tx);
  }
}

pub fn remove_group(group_id: &str) {
  RUNNING_GROUPS.lock().unwrap().remove(group_id);
  STOP_REQUESTS.lock().unwrap().remove(group_id);
}

pub fn subscribe_group(group_id: &str) -> watch::Receiver<bool> {
  let mut map = RUNNING_GROUPS.lock().unwrap();
  if let Some(tx) = map.get(group_id) {
    tx.subscribe()
  } else {
    let (tx, rx) = watch::channel(true);
    map.insert(group_id.to_string(), tx);
    rx
  }
}

pub fn is_group_running(group_id: &str) -> bool {
  RUNNING_GROUPS
    .lock()
    .unwrap()
    .get(group_id)
    .map(|tx| *tx.borrow())
    .unwrap_or(false)
}
