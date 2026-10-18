use crate::logging::LogStoreState;
use crate::models::download::DownloadSection;
use crate::models::progress::MediaDestinationPath;
use crate::models::{
  MediaDestination, MediaDiagnosticPayload, MediaFatalPayload, MediaProgressComplete,
  MediaProgressStage, ProgressEvent, ProgressStage,
};
use crate::parsers::ytdlp_error::{DiagnosticMatcher, YtdlpErrorParser};
use crate::parsers::ytdlp_progress::{YtdlpProgressParser, progress_category_for_track_type};
use crate::paths::PathsManager;
use crate::runners::live_recording::{destination_from_line, save_recording, wait_until_written};
use crate::runners::ytdlp_runner::{YtdlpChild, YtdlpCommandEvent, YtdlpRunner};
use crate::scheduling::download_pipeline::DownloadEntry;
use crate::scheduling::group_state::{is_stop_requested, subscribe_group};
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::watch;

#[derive(Debug)]
pub enum YtdlpDownloadError {
  InvalidArguments(String),
  InvalidDiagnosticRules(String),
  SpawnFailed(String),
  RunnerError(String),
  NonZeroExit(i32),
  EventStreamEnded,
}

impl fmt::Display for YtdlpDownloadError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidArguments(e) => write!(f, "Invalid yt-dlp arguments: {e}"),
      Self::InvalidDiagnosticRules(e) => write!(f, "Invalid diagnostic rules: {e}"),
      Self::SpawnFailed(e) => write!(f, "Failed to spawn yt-dlp: {e}"),
      Self::RunnerError(e) => write!(f, "yt-dlp runner error: {e}"),
      Self::NonZeroExit(code) => write!(f, "yt-dlp exited with code {code}"),
      Self::EventStreamEnded => write!(f, "yt-dlp event stream ended unexpectedly"),
    }
  }
}

impl std::error::Error for YtdlpDownloadError {}

pub async fn run_ytdlp_download(
  app: AppHandle,
  entry: DownloadEntry,
) -> Result<(), YtdlpDownloadError> {
  let runner = YtdlpRunner::new(&app)
    .with_progress_args()
    .with_performance_args()
    .with_live_args(entry.is_live, entry.overrides.as_ref())
    .with_network_args(entry.overrides.as_ref())
    .with_auth_args(entry.overrides.as_ref())
    .with_subtitle_args(entry.overrides.as_ref(), entry.subtitle_inventory.as_ref())
    .with_sponsorblock_args(entry.overrides.as_ref())
    .with_format_args(&entry.format, entry.overrides.as_ref())
    .with_input_filter_args(entry.overrides.as_ref())
    .with_input_args(entry.overrides.as_ref())
    .with_playlist_item(entry.playlist_item);
  let output_args = match runner.output_args(&entry.format, entry.overrides.as_ref()) {
    Ok(args) => args,
    Err(err) => {
      emit_internal_fatal(
        &app,
        &entry,
        format!("Invalid download configuration: {err}"),
        Some(err.clone()),
      );
      return Err(YtdlpDownloadError::InvalidArguments(err));
    }
  };
  let runner = runner
    .with_args(output_args)
    .with_location_args(
      &entry.format.track_type,
      &entry.template_context,
      entry.overrides.as_ref(),
    )
    .with_url(&entry.url);

  static RULES_JSON: &str = include_str!("../diagnostic_rules.json");
  let matcher = match DiagnosticMatcher::from_json(RULES_JSON) {
    Ok(matcher) => matcher,
    Err(err) => {
      emit_internal_fatal(
        &app,
        &entry,
        format!("Invalid diagnostic rules: {err}"),
        Some(err.to_string()),
      );
      return Err(YtdlpDownloadError::InvalidDiagnosticRules(err.to_string()));
    }
  };

  let error_parser = YtdlpErrorParser::new(&entry.id, &entry.group_id, matcher);
  let partial_download_duration_secs = entry
    .overrides
    .as_ref()
    .and_then(|overrides| overrides.output.as_ref())
    .and_then(|output| output.partial_download.as_ref())
    .and_then(|partial| partial.section.as_ref())
    .and_then(download_section_duration_secs)
    .filter(|value| *value > 0.0);
  let mut progress_parser = YtdlpProgressParser::new(
    &entry.id,
    &entry.group_id,
    progress_category_for_track_type(&entry.format.track_type),
    partial_download_duration_secs,
  )
  .live(entry.is_live);

  let (mut rx, child) = match runner.spawn() {
    Ok(result) => result,
    Err(err) => {
      let message = format!("Failed to spawn yt-dlp: {err}");
      let _ = app.emit(
        "media_fatal",
        MediaFatalPayload::with_exit(entry.group_id.clone(), entry.id.clone(), 1, message),
      );
      return Err(YtdlpDownloadError::SpawnFailed(err));
    }
  };
  let mut cancel_rx = subscribe_group(&entry.group_id);
  // The files a live recording writes to, to save them if it is stopped.
  let mut destinations: Vec<PathBuf> = Vec::new();

  loop {
    tokio::select! {
      event = rx.recv() => {
        let Some(event) = event else {
          break;
        };

        if is_cancelled_now(&cancel_rx) {
          return stop_download(&app, &entry, &child, &mut rx, &destinations).await;
        }

        let log_state = app.state::<LogStoreState>();

        match event {
          YtdlpCommandEvent::Stdout(line) => {
            let line_str = String::from_utf8_lossy(&line);
            if entry.is_live
              && let Some(destination) = destination_from_line(&line_str)
              && !destinations.contains(&destination)
            {
              destinations.push(destination);
            }
            store_log_line(&line_str, &entry, log_state, &app);
            parse_progress_line(&line_str, &mut progress_parser, &app);
            parse_error_line(&line_str, &error_parser, &app);
          }
          YtdlpCommandEvent::Stderr(line) => {
            let line_str = String::from_utf8_lossy(&line);
            store_log_line(&line_str, &entry, log_state, &app);
            parse_progress_line(&line_str, &mut progress_parser, &app);
            parse_error_line(&line_str, &error_parser, &app);
          }
          YtdlpCommandEvent::Terminated(term) => {
            if term.code == Some(0) {
              let _ = app.emit(
                "media_complete",
                MediaProgressComplete {
                  id: entry.id.clone(),
                  group_id: entry.group_id.clone(),
                },
              );
              return Ok(());
            }

            let exit = term.code.unwrap_or(1);
            let _ = app.emit(
              "media_fatal",
              MediaFatalPayload::with_exit(
                entry.group_id.clone(),
                entry.id.clone(),
                exit,
                format!("Download failed for group {}", entry.group_id),
              ),
            );
            return Err(YtdlpDownloadError::NonZeroExit(exit));
          }
          YtdlpCommandEvent::Error(err) => {
            let msg = format!("Download failed for group {}: {}", entry.group_id, err);
            emit_internal_fatal(&app, &entry, msg.clone(), Some(err.clone()));

            return Err(YtdlpDownloadError::RunnerError(err));
          }
        }
      }
      _ = cancel_rx.changed() => {
        if is_cancelled_now(&cancel_rx) {
          return stop_download(&app, &entry, &child, &mut rx, &destinations).await;
        }
      }
    }
  }

  emit_internal_fatal(
    &app,
    &entry,
    "yt-dlp event stream ended unexpectedly".to_string(),
    None,
  );
  Err(YtdlpDownloadError::EventStreamEnded)
}

/// Cancels a download. A live recording that the user stopped is saved.
async fn stop_download(
  app: &AppHandle,
  entry: &DownloadEntry,
  child: &YtdlpChild,
  rx: &mut UnboundedReceiver<YtdlpCommandEvent>,
  destinations: &[PathBuf],
) -> Result<(), YtdlpDownloadError> {
  let _ = child.kill_tree();
  if !(entry.is_live && is_stop_requested(&entry.group_id)) {
    tracing::info!("Cancelled processing for group_id {}", entry.group_id);
    return Ok(());
  }

  tracing::info!("Saving the live recording of group_id {}", entry.group_id);
  let _ = app.emit(
    "media_progress_stage",
    MediaProgressStage {
      id: entry.id.clone(),
      group_id: entry.group_id.clone(),
      stage: ProgressStage::Finalizing,
    },
  );

  // yt-dlp has to exit first, so that its files are closed.
  let exited = async {
    while let Some(event) = rx.recv().await {
      if matches!(
        event,
        YtdlpCommandEvent::Terminated(_) | YtdlpCommandEvent::Error(_)
      ) {
        break;
      }
    }
  };
  let _ = tokio::time::timeout(Duration::from_secs(15), exited).await;
  wait_until_written(destinations).await;

  let ffmpeg = app
    .state::<PathsManager>()
    .bin_dir()
    .join(if cfg!(windows) {
      "ffmpeg.exe"
    } else {
      "ffmpeg"
    });
  match save_recording(&ffmpeg, destinations).await {
    Ok(path) => {
      let _ = app.emit(
        "media_destination",
        MediaDestination {
          id: entry.id.clone(),
          group_id: entry.group_id.clone(),
          destination: MediaDestinationPath {
            confidence: 100,
            path: path.to_string_lossy().into_owned(),
          },
          is_merged: true,
        },
      );
      let _ = app.emit(
        "media_complete",
        MediaProgressComplete {
          id: entry.id.clone(),
          group_id: entry.group_id.clone(),
        },
      );
      Ok(())
    }
    Err(err) => {
      emit_internal_fatal(app, entry, err.clone(), None);
      Err(YtdlpDownloadError::RunnerError(err))
    }
  }
}

fn emit_internal_fatal(
  app: &AppHandle,
  entry: &DownloadEntry,
  message: String,
  details: Option<String>,
) {
  let _ = app.emit(
    "media_fatal",
    MediaFatalPayload::internal(entry.group_id.clone(), entry.id.clone(), message, details),
  );
}

fn is_cancelled_now(cancel_rx: &watch::Receiver<bool>) -> bool {
  !*cancel_rx.borrow()
}

fn store_log_line(
  line: &str,
  entry: &DownloadEntry,
  log_state: State<LogStoreState>,
  app: &AppHandle,
) {
  // Progress lines of yt-dlp and ffmpeg: a live recording prints one every
  // half second.
  let unnumbered = line.split_once(": ").map_or(line, |(number, rest)| {
    if number.bytes().all(|b| b.is_ascii_digit()) {
      rest
    } else {
      line
    }
  });
  if line.is_empty() || unnumbered.starts_with("RAW") || line.starts_with("frame=") {
    return;
  }
  let mut store = log_state.write();
  store.append_line(app, &entry.group_id, line);
}

fn parse_progress_line(line: &str, progress_parser: &mut YtdlpProgressParser, app: &AppHandle) {
  let progress_events = progress_parser.parse_line(line);

  for progress_event in progress_events {
    match progress_event {
      ProgressEvent::Destination(destination) => {
        app.emit("media_destination", destination).ok();
      }
      ProgressEvent::Progress(progress) => {
        app.emit("media_progress", progress).ok();
      }
      ProgressEvent::StageChange(progress) => {
        app.emit("media_progress_stage", progress).ok();
      }
    }
  }
}

fn parse_error_line(line: &str, error_parser: &YtdlpErrorParser, app: &AppHandle) {
  if let Some(event) = error_parser.parse_line(line) {
    app
      .emit(
        "media_diagnostic",
        MediaDiagnosticPayload::from_diagnostic_event(event),
      )
      .ok();
  }
}

fn download_section_duration_secs(section: &DownloadSection) -> Option<f64> {
  let start = parse_clock_secs(&section.start)?;
  let end = parse_clock_secs(&section.end)?;
  (end > start).then_some(end - start)
}

fn parse_clock_secs(value: &str) -> Option<f64> {
  let mut total = 0.0;
  let parts = value.trim().split(':').collect::<Vec<_>>();
  if parts.len() != 3 {
    return None;
  }

  for (index, part) in parts.iter().enumerate() {
    let component = part.trim().parse::<f64>().ok()?;
    total += component
      * match index {
        0 => 3600.0,
        1 => 60.0,
        2 => 1.0,
        _ => unreachable!(),
      };
  }

  Some(total)
}
