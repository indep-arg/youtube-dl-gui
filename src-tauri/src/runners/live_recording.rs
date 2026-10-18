//! Saves what a live recording got so far when the user stops it.
//!
//! Stopping kills yt-dlp, which leaves `.part` files behind: one MPEG-TS file
//! when ffmpeg records the stream, or a video and an audio file with
//! `--live-from-start`. Both can be copied into a normal file with ffmpeg.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use crate::runners::ytdlp_process::configure_command;

/// The file yt-dlp is writing to, from a `[download] Destination: …` line.
pub fn destination_from_line(line: &str) -> Option<PathBuf> {
  let path = line.strip_prefix("[download] Destination:")?.trim();
  (!path.is_empty()).then(|| PathBuf::from(path))
}

/// The name of the saved recording. With several streams their names carry the
/// format (`Title.f397.mp4`, `Title.f140.mp4`): the saved file is `Title.mp4`,
/// or `Title.mkv` when the streams use different containers.
pub fn saved_path(destinations: &[PathBuf]) -> Option<PathBuf> {
  let first = destinations.first()?;
  if destinations.len() == 1 {
    return Some(first.clone());
  }

  let extensions: Vec<_> = destinations.iter().map(|d| d.extension()).collect();
  let extension = if extensions.iter().all(|e| *e == extensions[0]) {
    extensions[0]?.to_string_lossy().into_owned()
  } else {
    "mkv".to_string()
  };

  let stem = first.file_stem()?.to_string_lossy();
  let stem = match stem.rsplit_once('.') {
    Some((title, format)) if is_format_id(format) => title.to_string(),
    _ => stem.into_owned(),
  };
  Some(first.with_file_name(format!("{stem}.{extension}")))
}

fn is_format_id(part: &str) -> bool {
  part.len() > 1
    && part.starts_with('f')
    && part[1..]
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The file that holds what was recorded for one destination.
fn recorded_file(destination: &Path) -> Option<PathBuf> {
  let part = part_path(destination);
  if is_non_empty(&part) {
    Some(part)
  } else if is_non_empty(destination) {
    Some(destination.to_path_buf())
  } else {
    None
  }
}

fn part_path(destination: &Path) -> PathBuf {
  let mut name = destination.as_os_str().to_owned();
  name.push(".part");
  PathBuf::from(name)
}

fn is_non_empty(path: &Path) -> bool {
  std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0)
}

/// Files yt-dlp leaves next to a stopped download: `X.part`, `X.ytdl` and the
/// fragments `X.part-Frag12.part`.
fn leftovers(destination: &Path) -> Vec<PathBuf> {
  let mut files = vec![part_path(destination)];
  let mut ytdl = destination.as_os_str().to_owned();
  ytdl.push(".ytdl");
  files.push(PathBuf::from(ytdl));

  if let (Some(dir), Some(name)) = (destination.parent(), destination.file_name()) {
    let fragment_prefix = format!("{}.part-Frag", name.to_string_lossy());
    if let Ok(entries) = std::fs::read_dir(dir) {
      for entry in entries.flatten() {
        if entry
          .file_name()
          .to_string_lossy()
          .starts_with(&fragment_prefix)
        {
          files.push(entry.path());
        }
      }
    }
  }
  files
}

/// Waits until the recorded files stop growing: ffmpeg may still be writing
/// its last bytes right after yt-dlp exits.
pub async fn wait_until_written(destinations: &[PathBuf]) {
  let size = || -> u64 {
    destinations
      .iter()
      .filter_map(|d| recorded_file(d))
      .filter_map(|f| std::fs::metadata(f).ok())
      .map(|m| m.len())
      .sum()
  };
  let mut last = size();
  for _ in 0..20 {
    tokio::time::sleep(Duration::from_millis(250)).await;
    let now = size();
    if now == last {
      return;
    }
    last = now;
  }
}

/// Copies the recorded streams into one file with ffmpeg and removes the
/// partial files. Returns the path of the saved file.
pub async fn save_recording(ffmpeg: &Path, destinations: &[PathBuf]) -> Result<PathBuf, String> {
  let inputs: Vec<PathBuf> = destinations
    .iter()
    .filter_map(|d| recorded_file(d))
    .collect();
  if inputs.is_empty() {
    return Err("Nothing was recorded yet.".into());
  }
  let target = saved_path(destinations).ok_or("The recording has no file name.")?;
  let extension = target
    .extension()
    .map(|e| e.to_string_lossy().into_owned())
    .unwrap_or_else(|| "mkv".into());
  // ffmpeg picks the container from the extension, so keep it last.
  let temporary = target.with_extension(format!("saving.{extension}"));

  let mut command = std::process::Command::new(ffmpeg);
  command.args(["-v", "error", "-y"]);
  for input in &inputs {
    command.arg("-i").arg(input);
  }
  // Only picture and sound: a stream may carry data tracks (e.g. ID3 tags)
  // that other containers cannot hold.
  for index in 0..inputs.len() {
    command
      .arg("-map")
      .arg(format!("{index}:v?"))
      .arg("-map")
      .arg(format!("{index}:a?"));
  }
  command
    .args(["-c", "copy"])
    .arg(&temporary)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
  configure_command(&mut command).map_err(|e| e.to_string())?;
  let mut command = tokio::process::Command::from(command);
  command.kill_on_drop(true);

  let output = command
    .output()
    .await
    .map_err(|e| format!("Could not start ffmpeg: {e}"))?;
  if !output.status.success() {
    let _ = tokio::fs::remove_file(&temporary).await;
    let error = String::from_utf8_lossy(&output.stderr);
    let error = error.lines().last().unwrap_or("").trim();
    return Err(format!("ffmpeg could not save the recording: {error}"));
  }

  tokio::fs::rename(&temporary, &target)
    .await
    .map_err(|e| format!("Could not move the recording into place: {e}"))?;
  for destination in destinations {
    for file in leftovers(destination) {
      if file != target {
        let _ = tokio::fs::remove_file(file).await;
      }
    }
  }
  Ok(target)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reads_the_destination() {
    assert_eq!(
      destination_from_line("[download] Destination: /videos/News.mp4"),
      Some(PathBuf::from("/videos/News.mp4"))
    );
    assert_eq!(
      destination_from_line("[info] Downloading 1 format(s): 0"),
      None
    );
  }

  #[test]
  fn one_stream_keeps_its_name() {
    let destinations = [PathBuf::from("/v/News live.mp4")];
    assert_eq!(
      saved_path(&destinations),
      Some(PathBuf::from("/v/News live.mp4"))
    );
  }

  #[test]
  fn separate_streams_lose_the_format_in_the_name() {
    let same = [
      PathBuf::from("/v/News.f397.mp4"),
      PathBuf::from("/v/News.f140.mp4"),
    ];
    assert_eq!(saved_path(&same), Some(PathBuf::from("/v/News.mp4")));

    let mixed = [
      PathBuf::from("/v/A.b.f303.webm"),
      PathBuf::from("/v/A.b.f140.m4a"),
    ];
    assert_eq!(saved_path(&mixed), Some(PathBuf::from("/v/A.b.mkv")));
  }

  #[test]
  fn only_format_ids_are_removed() {
    assert!(is_format_id("f397"));
    assert!(is_format_id("f303-1"));
    assert!(!is_format_id("f"));
    assert!(!is_format_id("final cut"));
  }

  fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ovd-live-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn finds_the_files_a_stopped_download_leaves() {
    let dir = temp_dir();
    let destination = dir.join("News.f397.mp4");
    for name in [
      "News.f397.mp4.part",
      "News.f397.mp4.ytdl",
      "News.f397.mp4.part-Frag50.part",
      "Other.mp4.part",
    ] {
      std::fs::write(dir.join(name), b"x").unwrap();
    }

    let mut found: Vec<_> = leftovers(&destination)
      .into_iter()
      .filter(|f| f.exists())
      .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
      .collect();
    found.sort();
    assert_eq!(
      found,
      [
        "News.f397.mp4.part",
        "News.f397.mp4.part-Frag50.part",
        "News.f397.mp4.ytdl"
      ]
    );
    assert_eq!(
      recorded_file(&destination),
      Some(dir.join("News.f397.mp4.part"))
    );
    std::fs::remove_dir_all(dir).unwrap();
  }

  #[tokio::test]
  async fn nothing_recorded_is_an_error() {
    let dir = temp_dir();
    let result = save_recording(Path::new("ffmpeg"), &[dir.join("News.mp4")]).await;
    assert_eq!(result, Err("Nothing was recorded yet.".to_string()));
    std::fs::remove_dir_all(dir).unwrap();
  }
}
