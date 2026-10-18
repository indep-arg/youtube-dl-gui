mod chapters;
mod codecs;
mod common;
mod formats;
mod tracks;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;

use crate::models::{ParsedMedia, ParsedSingleVideo, SubtitleInventory, YtdlpInfo};
use chapters::process_chapters;
use formats::{ProcessedFormats, process_formats};
use std::collections::{HashMap, HashSet};
use tracks::auto_track;

pub(crate) use common::i64_to_u64;

pub fn parse_single(info: YtdlpInfo, id: String) -> ParsedMedia {
  let subtitle_inventory =
    parse_subtitle_inventory(info.subtitles.as_ref(), info.automatic_captions.as_ref());
  let ProcessedFormats {
    video_codecs,
    audio_codecs,
    media_formats,
    audio_tracks,
    video_tracks,
  } = if let Some(formats) = &info.formats {
    process_formats(formats)
  } else {
    ProcessedFormats {
      video_codecs: Vec::new(),
      audio_codecs: Vec::new(),
      media_formats: Vec::new(),
      audio_tracks: vec![auto_track()],
      video_tracks: vec![auto_track()],
    }
  };

  let detected = media_formats.len();
  let provided = info.formats.as_ref().map_or(0, Vec::len);
  let loss_pct = if provided == 0 {
    0.0
  } else {
    100.0 * (provided - detected) as f64 / provided as f64
  };

  tracing::debug!(
    "Detected unique formats: {} out of {} provided ({:.1}% filtered).",
    detected,
    provided,
    loss_pct,
  );

  // A scheduled stream is recorded like a live one once it starts.
  let is_upcoming = info.live_status.as_deref() == Some("is_upcoming");

  ParsedMedia::Single(ParsedSingleVideo {
    id,
    url: info.webpage_url,
    title: info.title,
    thumbnail: info.thumbnail,
    uploader: info.uploader,
    uploader_id: info.uploader_id,
    duration: info.duration,
    views: i64_to_u64(info.view_count),
    likes: i64_to_u64(info.like_count),
    dislikes: i64_to_u64(info.dislike_count),
    rating: info.average_rating,
    description: info.description,
    extractor: info.extractor_key,
    comments: i64_to_u64(info.comment_count),
    filesize: i64_to_u64(info.filesize.or(info.filesize_approx)),
    is_live: info.is_live.unwrap_or(false) || is_upcoming,
    scheduled_at: info.release_timestamp.filter(|_| is_upcoming),
    video_codecs,
    audio_codecs,
    video_tracks,
    audio_tracks,
    formats: media_formats,
    subtitle_inventory,
    chapters: process_chapters(info.chapters.as_deref()),
    playlist_item: None,
  })
}

fn parse_subtitle_inventory(
  manual: Option<&HashMap<String, Vec<crate::models::ytdlp::YtdlpSubtitle>>>,
  automatic: Option<&HashMap<String, Vec<crate::models::ytdlp::YtdlpSubtitle>>>,
) -> SubtitleInventory {
  SubtitleInventory {
    manual_languages: collect_subtitle_languages(manual),
    auto_languages: collect_subtitle_languages(automatic),
  }
}

fn collect_subtitle_languages(
  subtitles: Option<&HashMap<String, Vec<crate::models::ytdlp::YtdlpSubtitle>>>,
) -> Vec<String> {
  let Some(subtitles) = subtitles else {
    return Vec::new();
  };

  let mut seen = HashSet::new();
  let mut languages = Vec::new();

  for (language, tracks) in subtitles {
    let normalized = language.trim().to_ascii_lowercase();
    if normalized.is_empty() || tracks.is_empty() || is_chat_track(&normalized) {
      continue;
    }
    if tracks
      .iter()
      .all(|track| track.ext.as_deref().is_none_or(str::is_empty))
    {
      continue;
    }
    if seen.insert(normalized.clone()) {
      languages.push(normalized);
    }
  }

  languages.sort();
  languages
}

/// yt-dlp lists the chat of a live stream (YouTube) or VOD (Twitch) as a
/// subtitle. It is not one, and on a live stream it downloads until the end.
fn is_chat_track(language: &str) -> bool {
  matches!(language, "live_chat" | "rechat")
}
