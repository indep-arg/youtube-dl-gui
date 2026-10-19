use crate::logging::LogStoreState;
use crate::models::download::FormatOptions;
use crate::models::{MediaDiagnosticPayload, MediaFatalPayload, ParsedMedia, TrackType};
use crate::parsers::ytdlp_error::{YtdlpErrorParser, shipped_matcher};
use crate::parsers::ytdlp_info::{parse_ytdlp_info, unwrap_selected_playlist_entry};
use crate::runners::ytdlp_runner::YtdlpRunner;
use crate::scheduling::fetch_pipeline::FetchEntry;
use crate::scheduling::group_state::RunReceiver;
use std::borrow::Cow;
use std::fmt;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug)]
pub enum YtdlpInfoFetchError {
  InvalidDiagnosticRules(String),
  RunnerFailed(String),
  NonZeroExit(i32),
  ParseFailed(String),
}

impl fmt::Display for YtdlpInfoFetchError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidDiagnosticRules(e) => write!(f, "Invalid diagnostic rules: {e}"),
      Self::RunnerFailed(e) => write!(f, "yt-dlp invocation failed: {e}"),
      Self::NonZeroExit(code) => write!(f, "yt-dlp exited with code {code}"),
      Self::ParseFailed(e) => write!(f, "Failed to parse yt-dlp output: {e}"),
    }
  }
}

impl std::error::Error for YtdlpInfoFetchError {}

pub async fn run_ytdlp_info_fetch(
  app: &AppHandle,
  entry: &FetchEntry,
  mut run: RunReceiver,
) -> Result<Option<ParsedMedia>, YtdlpInfoFetchError> {
  let id = entry.id.clone();
  let group_id = entry.group_id.clone();
  let url = entry.url.as_str();
  let overrides = entry.overrides.clone();
  let playlist_item = entry.playlist_item;
  let format = entry.format.clone().unwrap_or(FormatOptions {
    track_type: TrackType::Both,
    abr: None,
    height: None,
    fps: None,
    audio_encoding: None,
    video_encoding: None,
    audio_track: None,
    video_track: None,
  });
  let runner = |extra_args: &[&str]| {
    YtdlpRunner::new(app)
      .with_format_args(&format, None)
      .with_input_args(overrides.as_ref())
      .with_input_filter_args(overrides.as_ref())
      .with_auth_args(overrides.as_ref())
      .with_network_args(overrides.as_ref())
      .with_playlist_item(playlist_item)
      .with_args(["-J", "--flat-playlist"])
      .with_args(extra_args.iter().copied())
      .with_url(url)
  };

  let matcher = match shipped_matcher() {
    Ok(m) => m,
    Err(e) => {
      let _ = app.emit(
        "media_fatal",
        MediaFatalPayload::internal(
          group_id.clone(),
          id.clone(),
          format!("Invalid diagnostic rules: {e}"),
          None,
        ),
      );
      return Err(YtdlpInfoFetchError::InvalidDiagnosticRules(e.to_string()));
    }
  };

  let error_parser = YtdlpErrorParser::new(&id, &group_id, matcher);

  // `None` when the group was cancelled: there is nothing to report.
  let output = match runner(&[]).output_until_ended(&mut run).await {
    Ok(None) => return Ok(None),
    // A stream that has not started: yt-dlp only says so. Its details can be
    // fetched anyway, so that it can be queued and recorded when it starts.
    Ok(Some(o))
      if !o.status.success() && is_scheduled_stream(&String::from_utf8_lossy(&o.stderr)) =>
    {
      match runner(&["--ignore-no-formats-error"])
        .output_until_ended(&mut run)
        .await
      {
        Ok(None) => return Ok(None),
        Ok(Some(retry)) if retry.status.success() => retry,
        _ => o,
      }
    }
    Ok(Some(o)) => o,
    Err(e) => {
      let _ = app.emit(
        "media_fatal",
        MediaFatalPayload::with_exit(
          group_id.clone(),
          id.clone(),
          1,
          format!("Failed to spawn yt-dlp: {e}"),
        ),
      );
      return Err(YtdlpInfoFetchError::RunnerFailed(e));
    }
  };

  let stdout_text = String::from_utf8_lossy(&output.stdout);
  let stderr_text = String::from_utf8_lossy(&output.stderr);

  let log_state = app.state::<LogStoreState>();

  {
    let mut lines: Vec<&str> = Vec::new();
    if !stderr_text.is_empty() {
      lines.extend(stderr_text.lines());
    }
    if !stdout_text.is_empty() {
      lines.extend(stdout_text.lines());
    }

    {
      let mut store = log_state.write();
      store.append_lines(app, &group_id, stderr_text.lines());
    }

    let events = error_parser.parse_lines(lines);
    for event in events {
      let _ = app.emit(
        "media_diagnostic",
        MediaDiagnosticPayload::from_diagnostic_event(event),
      );
    }
  }

  let status_code: i32 = output.status.code().unwrap_or(1);
  if status_code != 0 {
    let parsed_events =
      error_parser.parse_lines(stderr_text.lines().chain(stdout_text.lines()).collect());

    // Truncate very large stderr payloads.
    let stderr_snippet: Cow<str> = if stderr_text.len() > STDERR_SNIPPET_BYTES {
      Cow::Owned(format!(
        "{}… (truncated, {} bytes total)",
        truncate_to_char_boundary(&stderr_text, STDERR_SNIPPET_BYTES),
        stderr_text.len()
      ))
    } else if stderr_text.is_empty() {
      if let Some(last_event) = parsed_events.last() {
        Cow::Owned(last_event.raw.clone())
      } else {
        Cow::Owned(format!("yt-dlp exited with code {status_code}"))
      }
    } else {
      stderr_text.clone()
    };

    let _ = app.emit(
      "media_fatal",
      MediaFatalPayload::with_exit(
        group_id.clone(),
        id.clone(),
        status_code,
        stderr_snippet.into_owned(),
      ),
    );
    return Err(YtdlpInfoFetchError::NonZeroExit(status_code));
  }

  let selected_entry = playlist_item.and_then(|_| unwrap_selected_playlist_entry(&stdout_text));
  let json = selected_entry.as_deref().unwrap_or(&stdout_text);

  match parse_ytdlp_info(json, id.clone()) {
    Ok(media) => Ok(Some(media)),
    Err(e) => {
      let _ = app.emit(
        "media_fatal",
        MediaFatalPayload::with_exit(
          group_id.clone(),
          id.clone(),
          1,
          format!("Failed to parse yt-dlp output: {e}"),
        ),
      );
      Err(YtdlpInfoFetchError::ParseFailed(e.to_string()))
    }
  }
}

const STDERR_SNIPPET_BYTES: usize = 8_192;

/// The longest start of `text` that fits in `max` bytes without splitting a
/// character: slicing in the middle of one panics, and panics abort the app.
fn truncate_to_char_boundary(text: &str, max: usize) -> &str {
  if text.len() <= max {
    return text;
  }
  let mut end = max;
  while !text.is_char_boundary(end) {
    end -= 1;
  }
  &text[..end]
}

/// yt-dlp's error for a live stream or premiere that has not started yet.
fn is_scheduled_stream(stderr: &str) -> bool {
  stderr.lines().any(|line| {
    line.starts_with("ERROR:")
      && (line.contains("This live event will begin in") || line.contains("Premieres in"))
  })
}

#[cfg(test)]
mod tests {
  use super::{is_scheduled_stream, truncate_to_char_boundary};

  #[test]
  fn truncation_never_splits_a_character() {
    let text = "€".repeat(4_000);
    let cut = truncate_to_char_boundary(&text, 8_192);
    assert_eq!(cut.len(), 8_190);
    assert!(cut.chars().all(|c| c == '€'));
    assert_eq!(truncate_to_char_boundary("short", 8_192), "short");
  }

  #[test]
  fn recognises_a_stream_that_has_not_started() {
    assert!(is_scheduled_stream(
      "ERROR: [youtube] UleiRdWFG7o: This live event will begin in 46 hours."
    ));
    assert!(is_scheduled_stream(
      "ERROR: [youtube] abc: Premieres in 2 hours"
    ));
    assert!(!is_scheduled_stream(
      "WARNING: [youtube] This live event will begin in 46 hours."
    ));
    assert!(!is_scheduled_stream(
      "ERROR: [youtube] abc: Video unavailable"
    ));
  }
}
