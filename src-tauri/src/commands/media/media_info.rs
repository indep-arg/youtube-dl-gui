use crate::FetchSender;
use crate::models::download::DownloadOverrides;
use crate::scheduling::dispatcher::DispatchRequest;
use crate::scheduling::fetch_pipeline::FetchRequest;
use crate::scheduling::group_state::FETCH_GROUPS;
use tauri::State;

#[tauri::command]
pub fn media_info(
  url: String,
  id: String,
  group_id: String,
  overrides: Option<DownloadOverrides>,
  pipeline: State<'_, FetchSender>,
) -> Result<String, String> {
  FETCH_GROUPS.start(&group_id);

  pipeline
    .0
    .send(DispatchRequest::Pipeline(FetchRequest::Initial {
      group_id: group_id.clone(),
      url,
      id,
      overrides: Box::new(overrides),
    }))
    .map_err(|e| e.to_string())?;

  Ok(group_id)
}
