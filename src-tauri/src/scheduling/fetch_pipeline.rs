use crate::models::download::{DownloadOverrides, FormatOptions};
use crate::models::error::DiagnosticLevel;
use crate::models::payloads::MediaAddWithFormatPayload;
use crate::models::{MediaAddPayload, MediaDiagnosticPayload, MediaFatalPayload, PlaylistEntry};
use crate::runners::ytdlp_info::run_ytdlp_info_fetch;
use crate::{
  models::{ParsedMedia, ParsedPlaylist},
  scheduling::concurrency::DynamicSemaphore,
  scheduling::dispatcher::{DispatchEntry, DispatchRequest, GenericDispatcher},
  scheduling::group_state::{FETCH_GROUPS, RunReceiver},
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Clone)]
pub struct FetchSender(pub UnboundedSender<DispatchRequest<FetchRequest>>);

#[derive(Clone)]
pub enum FetchRequest {
  Initial {
    group_id: String,
    id: String,
    url: String,
    overrides: Box<Option<DownloadOverrides>>,
  },
  Playlist {
    group_id: String,
    entries: Vec<PlaylistEntry>,
    overrides: Box<Option<DownloadOverrides>>,
  },
  Size {
    group_id: String,
    id: String,
    url: String,
    playlist_item: Option<u64>,
    format: FormatOptions,
  },
  SizePlaylist {
    group_id: String,
    playlist: ParsedPlaylist,
    format: FormatOptions,
  },
}

#[derive(Clone)]
pub struct FetchEntry {
  pub group_id: String,
  pub id: String,
  pub url: String,
  pub playlist_item: Option<u64>,
  pub total: usize,
  pub format: Option<FormatOptions>,
  pub overrides: Option<DownloadOverrides>,
}

impl DispatchEntry for FetchEntry {
  fn group_id(&self) -> &String {
    &self.group_id
  }
  fn group_key(&self) -> Option<&String> {
    None
  }
  fn set_numbering(&mut self, _autonumber: u64, _group_autonumber: Option<u64>) {}
}

pub fn setup_fetch_dispatcher(
  app: &AppHandle,
  sem: Arc<DynamicSemaphore>,
) -> GenericDispatcher<FetchRequest> {
  GenericDispatcher::start(
    app.clone(),
    sem,
    &FETCH_GROUPS,
    expand_fetch_request,
    |tx: UnboundedSender<DispatchRequest<FetchRequest>>,
     app: AppHandle,
     entry: FetchEntry,
     run: RunReceiver| async move {
      handle_fetch_entry(tx, app, entry, run).await;
    },
  )
}

fn expand_fetch_request(req: FetchRequest) -> Vec<FetchEntry> {
  match req {
    FetchRequest::Initial {
      group_id,
      id,
      url,
      overrides,
    } => {
      vec![FetchEntry {
        group_id,
        id,
        url,
        playlist_item: None,
        total: 1,
        format: None,
        overrides: *overrides,
      }]
    }
    FetchRequest::Playlist {
      group_id,
      entries,
      overrides,
    } => {
      let total = entries.len();
      entries
        .into_iter()
        .map(|e| FetchEntry {
          group_id: group_id.clone(),
          id: Uuid::new_v4().to_string(),
          url: e.video_url,
          playlist_item: e.playlist_item,
          total,
          format: None,
          overrides: *overrides.clone(),
        })
        .collect()
    }
    FetchRequest::Size {
      group_id,
      id,
      url,
      playlist_item,
      format,
    } => {
      vec![FetchEntry {
        group_id,
        id,
        url,
        playlist_item,
        total: 1,
        format: Some(format),
        overrides: None,
      }]
    }
    FetchRequest::SizePlaylist {
      group_id,
      playlist,
      format,
    } => {
      let total = playlist.entries.len();
      playlist
        .entries
        .into_iter()
        .map(|e| FetchEntry {
          group_id: group_id.clone(),
          id: Uuid::new_v4().to_string(),
          url: e.video_url,
          playlist_item: e.playlist_item,
          total,
          format: Some(format.clone()),
          overrides: None,
        })
        .collect()
    }
  }
}

async fn handle_fetch_entry(
  tx: UnboundedSender<DispatchRequest<FetchRequest>>,
  app: AppHandle,
  entry: FetchEntry,
  run: RunReceiver,
) {
  let result = run_ytdlp_info_fetch(&app, &entry, run).await;
  let FetchEntry {
    group_id,
    id,
    url,
    playlist_item,
    total,
    format,
    ..
  } = entry;

  let result = match result {
    Ok(v) => v,
    Err(e) => {
      tracing::warn!(
        fetch_id = %id,
        group_id = %group_id,
        url = %url,
        error = %e,
        "run_ytdlp_info_fetch failed"
      );

      None
    }
  };

  match result {
    Some(ParsedMedia::Single(mut single)) => {
      single.playlist_item = playlist_item;
      if let Some(format) = format {
        let payload = MediaAddWithFormatPayload {
          group_id: group_id.clone(),
          total,
          item: single,
          format,
        };
        let _ = app.emit("media_size", payload);
      } else {
        let payload = MediaAddPayload {
          group_id: group_id.clone(),
          total,
          item: single,
        };
        let _ = app.emit("media_add", payload);
      }
    }
    Some(ParsedMedia::Playlist(pl)) if is_empty_playlist(&pl) => {
      report_empty_playlist(&app, &group_id, &id);
    }
    Some(ParsedMedia::Playlist(pl)) => {
      if let Some(format) = format {
        let _ = tx.send(DispatchRequest::Pipeline(FetchRequest::SizePlaylist {
          group_id: group_id.clone(),
          playlist: pl,
          format,
        }));
      } else {
        let payload = MediaAddPayload {
          group_id,
          total: pl.entries.len(),
          item: pl,
        };
        let _ = app.emit("media_add", payload);
      }
    }
    None => {
      // Do nothing if no parsed result is returned. The events have already been sent.
    }
  }
}

fn is_empty_playlist(playlist: &ParsedPlaylist) -> bool {
  playlist.entries.is_empty()
}

/// An empty playlist has nothing to choose from, so it is shown as an error
/// (with a translated message for the `playlistEmpty` code).
fn report_empty_playlist(app: &AppHandle, group_id: &str, id: &str) {
  const MESSAGE: &str = "This playlist has no videos, or none of them can be downloaded.";
  let diagnostic = MediaDiagnosticPayload {
    group_id: group_id.to_string(),
    id: id.to_string(),
    level: DiagnosticLevel::Error,
    code: "playlistEmpty".into(),
    component: None,
    message: MESSAGE.into(),
    raw: String::new(),
    timestamp: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_millis())
      .unwrap_or(0),
  };
  let _ = app.emit("media_diagnostic", diagnostic);
  let fatal = MediaFatalPayload::with_exit(group_id.to_string(), id.to_string(), 1, MESSAGE.into());
  let _ = app.emit("media_fatal", fatal);
}
