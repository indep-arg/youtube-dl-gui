//! Whether the work of a group should go on.
//!
//! The fetch and download pipelines keep a registry each, so finishing or
//! cleaning up one never touches the other: the download of a playlist must
//! not be cancelled because fetching its entries just ended.
//!
//! Each start of a group is a run with its own channel. Work queued for a run
//! keeps a receiver of that channel, so once the run is cancelled, that work is
//! skipped even if the group is started again in the meantime.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex as StdMutex};
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
  Running,
  Cancelled,
  /// Stopped by the user: a live recording saves what it got.
  Stopped,
}

impl RunState {
  pub fn is_running(self) -> bool {
    self == Self::Running
  }
}

pub type RunReceiver = watch::Receiver<RunState>;

#[derive(Default)]
pub struct GroupRegistry {
  runs: StdMutex<HashMap<String, watch::Sender<RunState>>>,
}

pub static FETCH_GROUPS: LazyLock<GroupRegistry> = LazyLock::new(GroupRegistry::default);
pub static DOWNLOAD_GROUPS: LazyLock<GroupRegistry> = LazyLock::new(GroupRegistry::default);

impl GroupRegistry {
  /// Starts a run for the group, or keeps the current one if it is running.
  /// A run that was cancelled or stopped is replaced, which closes it for the
  /// work that was queued before.
  pub fn start(&self, group_id: &str) {
    let mut runs = self.runs.lock().unwrap();
    if runs
      .get(group_id)
      .is_some_and(|tx| tx.borrow().is_running())
    {
      return;
    }
    let (tx, _) = watch::channel(RunState::Running);
    runs.insert(group_id.to_string(), tx);
  }

  /// The current run of the group, if it is running.
  pub fn subscribe(&self, group_id: &str) -> Option<RunReceiver> {
    let runs = self.runs.lock().unwrap();
    runs
      .get(group_id)
      .filter(|tx| tx.borrow().is_running())
      .map(watch::Sender::subscribe)
  }

  pub fn cancel(&self, group_id: &str) {
    self.end(group_id, RunState::Cancelled);
  }

  pub fn stop(&self, group_id: &str) {
    self.end(group_id, RunState::Stopped);
  }

  // `send_replace` stores the state even when nothing is listening yet.
  fn end(&self, group_id: &str, state: RunState) {
    let runs = self.runs.lock().unwrap();
    if let Some(tx) = runs.get(group_id)
      && tx.borrow().is_running()
    {
      tx.send_replace(state);
    }
  }

  /// Forgets the group once its run has ended. A run that was started again
  /// in the meantime is kept.
  pub fn remove_if_ended(&self, group_id: &str) {
    let mut runs = self.runs.lock().unwrap();
    if runs
      .get(group_id)
      .is_some_and(|tx| !tx.borrow().is_running())
    {
      runs.remove(group_id);
    }
  }
}

/// Whether the run a receiver belongs to is still going. A run that was
/// replaced or forgotten has a closed channel and counts as ended.
pub fn is_running(run: &RunReceiver) -> bool {
  run.has_changed().is_ok() && run.borrow().is_running()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cancelling_is_stored_without_listeners() {
    let registry = GroupRegistry::default();
    registry.start("g");
    registry.cancel("g");
    assert!(registry.subscribe("g").is_none());
  }

  #[test]
  fn a_restarted_group_closes_the_old_run() {
    let registry = GroupRegistry::default();
    registry.start("g");
    let old = registry.subscribe("g").unwrap();
    registry.cancel("g");
    registry.start("g");
    let new = registry.subscribe("g").unwrap();
    assert!(!is_running(&old));
    assert!(is_running(&new));
  }

  #[test]
  fn cleanup_keeps_a_run_started_again() {
    let registry = GroupRegistry::default();
    registry.start("g");
    registry.cancel("g");
    registry.start("g");
    registry.remove_if_ended("g");
    assert!(registry.subscribe("g").is_some());
  }

  #[test]
  fn the_registries_are_independent() {
    let fetch = GroupRegistry::default();
    let download = GroupRegistry::default();
    fetch.start("g");
    download.start("g");
    fetch.cancel("g");
    fetch.remove_if_ended("g");
    assert!(download.subscribe("g").is_some());
  }

  #[tokio::test]
  async fn a_forgotten_run_wakes_its_listeners_once_with_the_end_state() {
    let registry = GroupRegistry::default();
    registry.start("g");
    let mut run = registry.subscribe("g").unwrap();
    registry.stop("g");
    registry.remove_if_ended("g");
    assert!(run.changed().await.is_ok());
    assert_eq!(*run.borrow(), RunState::Stopped);
    assert!(run.changed().await.is_err());
    assert!(!is_running(&run));
  }
}
