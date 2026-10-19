use crate::models::DownloadItem;
use crate::models::SubtitleInventory;
use crate::models::download::{DownloadOverrides, FormatOptions};
use crate::runners::template_context::TemplateContext;
use crate::runners::ytdlp_download::run_ytdlp_download;
use crate::scheduling::concurrency::DynamicSemaphore;
use crate::scheduling::dispatcher::{DispatchEntry, DispatchRequest, GenericDispatcher};
use crate::scheduling::group_state::DOWNLOAD_GROUPS;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct DownloadSender(pub UnboundedSender<DispatchRequest<DownloadRequest>>);

#[derive(Clone)]
pub enum DownloadRequest {
  Batch {
    group_id: String,
    items: Vec<DownloadItem>,
  },
}

#[derive(Clone)]
pub struct DownloadEntry {
  pub group_id: String,
  pub id: String,
  pub url: String,
  pub playlist_item: Option<u64>,
  pub format: FormatOptions,
  pub subtitle_inventory: Option<SubtitleInventory>,
  pub overrides: Option<DownloadOverrides>,
  pub template_context: TemplateContext,
  pub is_live: bool,
}

impl From<(DownloadItem, String)> for DownloadEntry {
  fn from(item: (DownloadItem, String)) -> Self {
    Self {
      group_id: item.1,
      id: item.0.id,
      url: item.0.url,
      playlist_item: item.0.playlist_item,
      format: item.0.format,
      subtitle_inventory: item.0.subtitle_inventory,
      overrides: item.0.overrides,
      template_context: item.0.template_context,
      is_live: item.0.is_live,
    }
  }
}

impl DispatchEntry for DownloadEntry {
  fn group_id(&self) -> &String {
    &self.group_id
  }
  fn group_key(&self) -> Option<&String> {
    self.template_context.values.get("playlist_id")
  }
  fn set_numbering(&mut self, autonumber: u64, group_autonumber: Option<u64>) {
    self
      .template_context
      .values
      .insert("autonumber".to_string(), autonumber.to_string());
    if let Some(group_autonumber) = group_autonumber {
      self.template_context.values.insert(
        "playlist_autonumber".to_string(),
        group_autonumber.to_string(),
      );
    }
  }
}

pub fn setup_download_dispatcher(
  app: &AppHandle,
  sem: Arc<DynamicSemaphore>,
) -> GenericDispatcher<DownloadRequest> {
  GenericDispatcher::start(
    app.clone(),
    sem,
    &DOWNLOAD_GROUPS,
    |req: DownloadRequest| match req {
      DownloadRequest::Batch { group_id, items } => items
        .into_iter()
        .map(|item| DownloadEntry::from((item, group_id.clone())))
        .collect(),
    },
    |_tx, app: AppHandle, entry: DownloadEntry, run| async move {
      tracing::info!("starting download id={} url={}", entry.id, entry.url);

      let download_id = entry.id.clone();
      let group_id = entry.group_id.clone();
      if let Err(e) = run_ytdlp_download(app, entry, run).await {
        tracing::warn!(
          download_id = %download_id,
          group_id = %group_id,
          error = %e,
          "Failed to run ytdlp download",
        );
      }
    },
  )
}
