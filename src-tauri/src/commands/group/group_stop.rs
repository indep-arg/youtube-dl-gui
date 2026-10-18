use crate::scheduling::group_state::stop_group;

/// Stops a live recording and saves what was recorded so far.
#[tauri::command]
pub fn group_stop(group_id: String) {
  stop_group(&group_id);
}
