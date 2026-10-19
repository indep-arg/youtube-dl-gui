use crate::logging::LogStoreState;
use crate::models::payloads::LogSnapshot;
use tauri::State;

#[tauri::command]
pub fn logging_subscribe(
  group_id: String,
  log_state: State<'_, LogStoreState>,
) -> Result<LogSnapshot, String> {
  let mut store = log_state.write();
  store.subscribe(group_id.clone());

  Ok(store.snapshot(&group_id))
}
