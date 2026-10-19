use crate::scheduling::dispatcher::DispatchRequest;
use crate::scheduling::download_pipeline::DownloadSender;
use crate::scheduling::group_state::DOWNLOAD_GROUPS;
use tauri::State;

/// Stops a live recording and saves what was recorded so far.
#[tauri::command]
pub fn group_stop(group_id: String, download_sender: State<'_, DownloadSender>) {
  DOWNLOAD_GROUPS.stop(&group_id);
  let _ = download_sender
    .0
    .send(DispatchRequest::Cleanup { group_id });
}
