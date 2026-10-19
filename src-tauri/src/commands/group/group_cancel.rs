use crate::logging::LogStoreState;
use crate::scheduling::dispatcher::DispatchRequest;
use crate::scheduling::download_pipeline::DownloadSender;
use crate::scheduling::fetch_pipeline::FetchSender;
use crate::scheduling::group_state::{DOWNLOAD_GROUPS, FETCH_GROUPS};
use tauri::State;

#[tauri::command]
pub fn group_cancel(
  group_id: String,
  log_state: State<'_, LogStoreState>,
  fetch_sender: State<'_, FetchSender>,
  download_sender: State<'_, DownloadSender>,
) {
  FETCH_GROUPS.cancel(&group_id);
  DOWNLOAD_GROUPS.cancel(&group_id);

  // Drops the work that is still queued and forgets the group.
  let _ = fetch_sender.0.send(DispatchRequest::Cleanup {
    group_id: group_id.clone(),
  });
  let _ = download_sender.0.send(DispatchRequest::Cleanup {
    group_id: group_id.clone(),
  });

  let mut store = log_state.write();
  store.remove_group(&group_id);
}
