//! A plain text summary of the app, the system and the installed tools, for bug
//! reports. It never contains links, file paths, user names or credentials.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use crate::binaries::binaries_manager::is_program;
use crate::runners::ytdlp_process::configure_command;

/// The tools the app installs, with the argument that prints their version.
const TOOLS: [(&str, &str); 5] = [
  ("yt-dlp", "--version"),
  ("ffmpeg", "-version"),
  ("ffprobe", "-version"),
  ("deno", "--version"),
  ("AtomicParsley", "--version"),
];

const VERSION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolState {
  Missing,
  NotAProgram,
  DoesNotRun(String),
  Runs(String),
}

#[derive(Debug, Clone)]
pub struct ToolReport {
  pub name: String,
  /// The version recorded when the app installed it.
  pub installed: Option<String>,
  pub state: ToolState,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
  pub app_version: String,
  pub system: String,
  pub install: String,
  pub webview: String,
  pub language: String,
  pub theme: String,
  pub update_tools: bool,
  pub update_app: bool,
  pub tools: Vec<ToolReport>,
}

pub fn render(report: &Report) -> String {
  let on_off = |value: bool| if value { "on" } else { "off" };
  let mut out = String::new();
  out.push_str(&format!("Open Video Downloader {}\n", report.app_version));
  out.push_str(&format!("System: {}\n", report.system));
  out.push_str(&format!("Install: {}\n", report.install));
  out.push_str(&format!("WebView: {}\n", report.webview));
  out.push_str(&format!(
    "Language: {}, theme: {}\n",
    report.language, report.theme
  ));
  out.push_str(&format!(
    "Updates: tools {}, app {}\n",
    on_off(report.update_tools),
    on_off(report.update_app)
  ));
  out.push_str("\nTools:\n");
  for tool in &report.tools {
    let installed = tool.installed.as_deref().unwrap_or("not installed");
    let state = match &tool.state {
      ToolState::Missing => "missing".to_string(),
      ToolState::NotAProgram => "the file is not a program".to_string(),
      ToolState::DoesNotRun(why) => format!("does not run ({why})"),
      ToolState::Runs(version) => format!("runs ({version})"),
    };
    out.push_str(&format!("- {} {}: {}\n", tool.name, installed, state));
  }
  out
}

/// Checks one installed tool: is it there, is it a program and does it start.
pub async fn check_tool(bin_dir: &Path, name: &str, version_arg: &str) -> ToolState {
  let file = if cfg!(windows) {
    bin_dir.join(format!("{name}.exe"))
  } else {
    bin_dir.join(name)
  };
  if tokio::fs::metadata(&file).await.is_err() {
    return ToolState::Missing;
  }
  if !is_program(&file).await {
    return ToolState::NotAProgram;
  }

  let mut command = std::process::Command::new(&file);
  command
    .arg(version_arg)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
  if let Err(e) = configure_command(&mut command) {
    return ToolState::DoesNotRun(e.to_string());
  }
  let mut command = tokio::process::Command::from(command);
  command.kill_on_drop(true);

  match tokio::time::timeout(VERSION_TIMEOUT, command.output()).await {
    Err(_) => ToolState::DoesNotRun("no answer".into()),
    Ok(Err(e)) => ToolState::DoesNotRun(e.to_string()),
    Ok(Ok(output)) if !output.status.success() => {
      ToolState::DoesNotRun(format!("exit status {}", output.status))
    }
    Ok(Ok(output)) => {
      let text = String::from_utf8_lossy(&output.stdout);
      ToolState::Runs(first_line(&text))
    }
  }
}

pub async fn check_tools(bin_dir: &Path, installed: &HashMap<String, String>) -> Vec<ToolReport> {
  let mut reports = Vec::new();
  for (name, arg) in TOOLS {
    reports.push(ToolReport {
      name: name.to_string(),
      installed: installed.get(name).cloned(),
      state: check_tool(bin_dir, name, arg).await,
    });
  }
  reports
}

/// The first line, cut to a sensible length (ffmpeg prints its whole build setup).
fn first_line(text: &str) -> String {
  let line = text
    .lines()
    .map(str::trim)
    .find(|l| !l.is_empty())
    .unwrap_or("");
  let mut short: String = line.chars().take(80).collect();
  if line.chars().count() > 80 {
    short.push('…');
  }
  short
}

/// The name of the system, e.g. "Fedora Linux 45 (KDE Plasma Desktop Edition)".
pub fn system_name() -> String {
  let base = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
  #[cfg(target_os = "linux")]
  {
    if let Ok(text) = std::fs::read_to_string("/etc/os-release")
      && let Some(name) = os_release_name(&text)
    {
      return format!("{base} ({name})");
    }
  }
  base
}

#[cfg(any(target_os = "linux", test))]
fn os_release_name(text: &str) -> Option<String> {
  text.lines().find_map(|line| {
    let value = line.strip_prefix("PRETTY_NAME=")?;
    Some(value.trim().trim_matches('"').to_string())
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn report() -> Report {
    Report {
      app_version: "2026.10.4".into(),
      system: "linux x86_64 (Fedora Linux 45)".into(),
      install: "AppImage".into(),
      webview: "2.54.1".into(),
      language: "es".into(),
      theme: "system".into(),
      update_tools: true,
      update_app: false,
      tools: vec![
        ToolReport {
          name: "yt-dlp".into(),
          installed: Some("2026.08.19".into()),
          state: ToolState::Runs("2026.08.19".into()),
        },
        ToolReport {
          name: "ffmpeg".into(),
          installed: Some("2026-10".into()),
          state: ToolState::NotAProgram,
        },
        ToolReport {
          name: "AtomicParsley".into(),
          installed: None,
          state: ToolState::Missing,
        },
      ],
    }
  }

  #[test]
  fn renders_every_part() {
    let text = render(&report());
    assert!(text.starts_with("Open Video Downloader 2026.10.4\n"));
    assert!(text.contains("System: linux x86_64 (Fedora Linux 45)"));
    assert!(text.contains("Install: AppImage"));
    assert!(text.contains("Updates: tools on, app off"));
    assert!(text.contains("- yt-dlp 2026.08.19: runs (2026.08.19)"));
    assert!(text.contains("- ffmpeg 2026-10: the file is not a program"));
    assert!(text.contains("- AtomicParsley not installed: missing"));
  }

  #[test]
  fn keeps_only_the_first_line_of_a_version() {
    let ffmpeg = "ffmpeg version N-127617-g94cba3150c-20261017 Copyright (c) 2000-2026 the FFmpeg developers\nbuilt with gcc";
    let line = first_line(ffmpeg);
    assert!(line.starts_with("ffmpeg version N-127617"));
    assert!(!line.contains("built with"));
    assert!(line.chars().count() <= 81);
  }

  #[test]
  fn reads_the_name_of_the_system() {
    let text =
      "NAME=\"Fedora Linux\"\nPRETTY_NAME=\"Fedora Linux 45 (KDE Plasma Desktop Edition)\"\n";
    assert_eq!(
      os_release_name(text).as_deref(),
      Some("Fedora Linux 45 (KDE Plasma Desktop Edition)")
    );
    assert_eq!(os_release_name("NAME=x"), None);
  }

  fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ovd-diagnostics-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[tokio::test]
  async fn a_missing_tool_and_an_archive_are_reported() {
    let dir = temp_dir();
    let ffmpeg = if cfg!(windows) {
      "ffmpeg.exe"
    } else {
      "ffmpeg"
    };
    std::fs::write(dir.join(ffmpeg), [0xfd, b'7', b'z', b'X', b'Z', 0]).unwrap();

    assert_eq!(
      check_tool(&dir, "ffmpeg", "-version").await,
      ToolState::NotAProgram
    );
    assert_eq!(
      check_tool(&dir, "deno", "--version").await,
      ToolState::Missing
    );
    std::fs::remove_dir_all(dir).unwrap();
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn a_working_tool_reports_its_version() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir();
    let tool = dir.join("yt-dlp");
    std::fs::write(&tool, "#!/bin/sh\necho 2026.08.19\n").unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(
      check_tool(&dir, "yt-dlp", "--version").await,
      ToolState::Runs("2026.08.19".into())
    );
    std::fs::remove_dir_all(dir).unwrap();
  }
}
