use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::SharedConfig;
use crate::binaries::binaries_extractor::{
  TarCompression, extract_tar, extract_tar_bundle, extract_tar_entries, extract_zip,
  extract_zip_bundle,
};
use crate::binaries::binaries_state::CheckResult;
use crate::paths::PathsManager;
use crate::state::config_models::YtDlpChannel;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Error, Manager, Wry};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const MANIFEST_URL: &str =
  "https://github.com/indep-arg/youtube-dl-gui/releases/download/manifest/manifest.json";
const MANIFEST_SIG_URL: &str =
  "https://github.com/indep-arg/youtube-dl-gui/releases/download/manifest/manifest.sig";
const MANIFEST_PUB_KEY: &str = "848e34a863b4c3e5358ec8d338cc065f0b6f025f93edb4fd0a25976b07f26c8e";

type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Deserialize, Serialize)]
struct Manifest {
  #[serde(rename = "generatedAt")]
  generated_at: String,
  tools: IndexMap<String, ToolInfo>,
  /// Other builds of some tools, e.g. `nightly` for yt-dlp. Older versions of the
  /// app ignore this field.
  #[serde(default)]
  channels: HashMap<String, ChannelInfo>,
}

#[derive(Deserialize, Serialize, Default)]
struct ChannelInfo {
  tools: IndexMap<String, ToolInfo>,
}

/// The tools that have other channels.
const CHANNEL_TOOLS: [&str; 1] = ["yt-dlp"];

struct ChannelTool<'a> {
  name: &'a str,
  info: &'a ToolInfo,
  /// The manifest has no build of the chosen channel (it could not be fetched
  /// that day): keep the installed copy instead of going back to stable.
  keep_installed: bool,
}

/// The tools of the manifest, with the builds of `channel` in place of the stable ones.
fn tools_for_channel(manifest: &Manifest, channel: YtDlpChannel) -> Vec<ChannelTool<'_>> {
  let builds = match channel {
    YtDlpChannel::Stable => None,
    other => Some(manifest.channels.get(other.as_str())),
  };
  manifest
    .tools
    .iter()
    .map(|(name, stable)| {
      let has_channel = CHANNEL_TOOLS.contains(&name.as_str());
      let (info, keep_installed) = match builds {
        Some(Some(channel)) if has_channel => (channel.tools.get(name).unwrap_or(stable), false),
        Some(None) if has_channel => (stable, true),
        _ => (stable, false),
      };
      ChannelTool {
        name: name.as_str(),
        info,
        keep_installed,
      }
    })
    .collect()
}

#[derive(Deserialize, Serialize)]
struct ToolInfo {
  version: String,
  files: HashMap<String, FileInfo>,
}

#[derive(Deserialize, Serialize)]
struct FileInfo {
  url: String,
  sha256: String,
  #[serde(default)]
  entry: Option<String>,
  #[serde(default)]
  bundle: Option<BundleInfo>,
}

#[derive(Deserialize, Serialize)]
struct BundleInfo {
  #[serde(default)]
  keep_folder: bool,
  #[serde(default)]
  folder_name: Option<String>,
  entry: String,
  #[serde(default)]
  rename_entry_to: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct Metadata {
  versions: HashMap<String, String>,
  is_locked: bool,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolError {
  tool: String,
  version: String,
  stage: String,
  error: String,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolResult {
  successes: Vec<String>,
  failures: Vec<ToolError>,
  error: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolStart {
  tool: String,
  version: String,
}

/// What the install loop knows about the other tools that come from the same archive.
#[derive(Default)]
struct SharedArchive {
  /// Another tool of this run will need the archive: do not delete it yet.
  keep_archive: bool,
  /// This tool was already extracted together with an earlier one.
  already_extracted: bool,
  /// Later tools in the same archive: `(tool, where its binary goes, its entry)`.
  later_tools: Vec<(String, PathBuf, String)>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolProgress {
  tool: String,
  total: u64,
  received: u64,
  /// The file was already downloaded for another tool in this run.
  reused: bool,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolComplete {
  tool: String,
}

/// How often a download reports its progress.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// A stalled connection fails instead of keeping the install screen waiting forever.
fn http_client() -> reqwest::Client {
  reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(30))
    .read_timeout(Duration::from_secs(60))
    .build()
    .unwrap_or_else(|_| reqwest::Client::new())
}

pub struct BinariesManager {
  app: AppHandle<Wry>,
  client: reqwest::Client,
  bin_dir: PathBuf,
}

impl BinariesManager {
  pub fn new(app: &AppHandle<Wry>) -> Self {
    let paths_manager = app.state::<PathsManager>();
    Self {
      app: app.clone(),
      client: http_client(),
      bin_dir: paths_manager.bin_dir().clone(),
    }
  }

  fn canonical_path(&self, tool: &str) -> Result<PathBuf, Error> {
    let base = &self.bin_dir;
    #[cfg(windows)]
    {
      Ok(base.join(format!("{tool}.exe")))
    }
    #[cfg(not(windows))]
    {
      Ok(base.join(tool))
    }
  }

  fn current_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
  }

  /// The file for this platform.
  ///
  /// Windows on ARM runs x64 programs, so a tool without an ARM build uses the x64
  /// one there. Linux on ARM cannot, so such a tool is left out.
  fn select_file<'a>(
    files: &'a HashMap<String, FileInfo>,
    arch: &'a str,
  ) -> Option<(&'a str, &'a FileInfo)> {
    if let Some(f) = files.get(arch) {
      return Some((arch, f));
    }
    if arch == "windows-aarch64" {
      return Self::select_file(files, "windows-x86_64");
    }
    None
  }

  async fn build_plan<'a>(
    &self,
    manifest: &'a Manifest,
    meta: &Metadata,
    arch: &str,
    allow: Option<&[String]>,
  ) -> Result<Vec<(&'a str, &'a ToolInfo)>, AnyError> {
    let channel = self
      .app
      .state::<SharedConfig>()
      .load()
      .update
      .yt_dlp_channel;
    let mut plan = Vec::new();
    for ChannelTool {
      name,
      info,
      keep_installed,
    } in tools_for_channel(manifest, channel)
    {
      if let Some(allow) = allow
        && !allow.iter().any(|n| n == name)
      {
        continue;
      }

      if Self::select_file(&info.files, arch).is_none() {
        continue;
      }

      let version_ok = meta.versions.get(name).is_some_and(|v| *v == info.version);
      let canonical = self.canonical_path(name)?;
      // A file that is there but is not a program (e.g. an archive that an older
      // version could not unpack) is installed again.
      let usable = is_program(&canonical).await;
      let kept = keep_installed && usable && meta.versions.contains_key(name);

      if (!version_ok || !usable) && !kept {
        plan.push((name, info));
      }
    }
    Ok(plan)
  }

  pub async fn check(&self) -> Result<CheckResult, AnyError> {
    let bin = &self.bin_dir;
    tokio::fs::create_dir_all(&bin).await?;

    let meta_path = bin.join("metadata.json");
    let meta = self.load_metadata(&meta_path).await?;
    if meta.is_locked {
      return Ok(CheckResult { tools: Vec::new() });
    }
    let arch = Self::current_platform();

    let manifest = self.fetch_manifest().await?;
    let plan = self.build_plan(&manifest, &meta, &arch, None).await?;
    let tools: Vec<String> = plan.iter().map(|(n, _)| (*n).to_string()).collect();

    Ok(CheckResult { tools })
  }

  pub async fn ensure(&self, allow: Option<&[String]>) -> Result<(), AnyError> {
    let bin = &self.bin_dir;
    tokio::fs::create_dir_all(&bin).await?;

    let meta_path = bin.join("metadata.json");
    let mut meta = self.load_metadata(&meta_path).await?;
    if meta.is_locked {
      return Ok(());
    }
    let arch = Self::current_platform();

    let manifest = self.fetch_manifest().await?;
    let plan = self.build_plan(&manifest, &meta, &arch, allow).await?;
    if plan.is_empty() {
      return Ok(());
    }

    let mut successes: Vec<String> = Vec::new();
    let mut failures: Vec<ToolError> = Vec::new();

    // ffmpeg and ffprobe can come from the same (large) archive: it is
    // downloaded once, kept until the last tool that needs it is installed, and
    // read once: the first tool also extracts the binaries of the later ones.
    let urls: Vec<Option<String>> = plan
      .iter()
      .map(|(_, info)| Self::select_file(&info.files, &arch).map(|(_, f)| f.url.clone()))
      .collect();
    let mut extracted_early: HashSet<String> = HashSet::new();

    for (index, (name, info)) in plan.iter().enumerate() {
      let shares_with_later = |later: usize| urls[index].is_some() && urls[later] == urls[index];
      let mut shared = SharedArchive {
        keep_archive: (index + 1..plan.len()).any(shares_with_later),
        already_extracted: extracted_early.contains(*name),
        later_tools: Vec::new(),
      };
      for later in (index + 1..plan.len()).filter(|l| shares_with_later(*l)) {
        let (later_name, later_info) = plan[later];
        let entry = Self::select_file(&later_info.files, &arch)
          .filter(|(_, f)| f.bundle.is_none())
          .and_then(|(_, f)| f.entry.clone());
        if let (Some(entry), Ok(canonical)) = (entry, self.canonical_path(later_name)) {
          shared
            .later_tools
            .push((later_name.to_string(), canonical, entry));
        }
      }

      match self
        .install_single_tool(bin, &arch, name, info, &shared)
        .await
      {
        Ok(extracted) => {
          extracted_early.extend(extracted);
          // only update meta + successes on fully successful install
          meta.versions.insert(name.to_string(), info.version.clone());
          successes.push(name.to_string());
        }
        Err(err) => {
          failures.push(err);
        }
      }
    }

    meta.is_locked = false;

    if let Err(e) = self.save_metadata(&meta_path, &meta).await {
      let msg = e.to_string();
      let _ = self.app.emit(
        "binary_update_complete",
        ToolResult {
          successes,
          failures: failures.clone(),
          error: Some(msg),
        },
      );
      return Err(e);
    }

    let _ = self.app.emit(
      "binary_update_complete",
      ToolResult {
        successes,
        failures: failures.clone(),
        error: None,
      },
    );

    if !failures.is_empty() {
      return Err("one or more tools failed to install".into());
    }

    Ok(())
  }

  async fn install_single_tool(
    &self,
    bin: &Path,
    arch: &str,
    name: &str,
    info: &ToolInfo,
    shared: &SharedArchive,
  ) -> Result<Vec<String>, ToolError> {
    let Some((_key, file)) = Self::select_file(&info.files, arch) else {
      return self.fail_stage(
        name,
        &info.version,
        "select_file",
        "no compatible file for current platform".to_string(),
      );
    };

    let url = &file.url;
    let Some(filename) = url.rsplit('/').next() else {
      return self.fail_stage(
        name,
        &info.version,
        "parse_filename",
        "missing file name in URL".to_string(),
      );
    };
    let dest = bin.join(filename);

    let _ = self.app.emit(
      "binary_download_start",
      ToolStart {
        tool: name.to_string(),
        version: info.version.to_string(),
      },
    );

    let mut extracted_for_others = Vec::new();
    if shared.already_extracted {
      // Its binary came out of the archive together with an earlier tool.
      self.report_reused(name, &dest).await;
      let _ = remove_archive(&dest, shared.keep_archive).await;
    } else {
      match self.download_and_verify(name, file, &dest, shared).await {
        Ok(written) => {
          extracted_for_others = shared
            .later_tools
            .iter()
            .filter(|(_, canonical, _)| written.contains(canonical))
            .map(|(later, _, _)| later.clone())
            .collect();
        }
        Err(e) => {
          return self.fail_stage(name, &info.version, "download_verify", e.to_string());
        }
      }
    }

    let canonical = match self.canonical_path(name) {
      Ok(path) => path,
      Err(e) => {
        return self.fail_stage(name, &info.version, "canonical_path", e.to_string());
      }
    };

    if !is_program(&canonical).await {
      let err = format!(
        "installed file is missing or not a program: {}",
        canonical.display()
      );
      return self.fail_stage(name, &info.version, "post_install_check", err);
    }

    let _ = self.app.emit(
      "binary_download_complete",
      ToolComplete {
        tool: name.to_string(),
      },
    );

    Ok(extracted_for_others)
  }

  /// Tells the install screen that this tool needs no download of its own.
  async fn report_reused(&self, tool: &str, archive: &Path) {
    let size = fs::metadata(archive).await.map(|m| m.len()).unwrap_or(0);
    let _ = self.app.emit(
      "binary_download_progress",
      ToolProgress {
        tool: tool.to_string(),
        received: size,
        total: size,
        reused: true,
      },
    );
  }

  fn fail_stage<T>(
    &self,
    name: &str,
    version: &str,
    stage: &str,
    msg: String,
  ) -> Result<T, ToolError> {
    self.emit_tool_error(name, version, stage, &msg);
    Err(ToolError {
      tool: name.to_string(),
      version: version.to_string(),
      stage: stage.to_string(),
      error: msg,
    })
  }

  async fn fetch_manifest(&self) -> Result<Manifest, AnyError> {
    let manifest_bytes = self
      .client
      .get(MANIFEST_URL)
      .send()
      .await?
      .error_for_status()?
      .bytes()
      .await?;

    let sig_b64 = self
      .client
      .get(MANIFEST_SIG_URL)
      .send()
      .await?
      .error_for_status()?
      .bytes()
      .await?;
    let sig_raw = base64::engine::general_purpose::STANDARD.decode(sig_b64)?;
    let sig = Signature::from_slice(&sig_raw)?;

    let key_vec = hex::decode(MANIFEST_PUB_KEY)?;
    let key_bytes: [u8; 32] = key_vec.as_slice().try_into().map_err(|_| {
      format!(
        "invalid public key length: expected 32, got {}",
        key_vec.len()
      )
    })?;

    let vk = VerifyingKey::from_bytes(&key_bytes)?;
    vk.verify(&manifest_bytes, &sig)?;

    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    Ok(manifest)
  }

  /// Returns the binaries of other tools that came out of the same archive.
  async fn download_and_verify(
    &self,
    tool: &str,
    file: &FileInfo,
    dest: &Path,
    shared: &SharedArchive,
  ) -> Result<Vec<PathBuf>, AnyError> {
    // Already downloaded for another tool in this run, and still intact.
    let reuse = file_sha256(dest).await.as_deref() == Some(file.sha256.as_str());
    self
      .download_file(tool, &file.url, &file.sha256, dest, reuse)
      .await?;
    self
      .install_from_download(
        tool,
        dest,
        file.entry.as_deref(),
        file.bundle.as_ref(),
        shared,
      )
      .await
  }

  /// Downloads `url` to `dest` and checks its hash. With `reuse`, the file is
  /// already there and only the progress is reported.
  async fn download_file(
    &self,
    tool: &str,
    url: &str,
    sha256: &str,
    dest: &Path,
    reuse: bool,
  ) -> Result<(), AnyError> {
    if reuse {
      let size = fs::metadata(dest).await?.len();
      let _ = self.app.emit(
        "binary_download_progress",
        ToolProgress {
          tool: tool.to_string(),
          received: size,
          total: size,
          reused: true,
        },
      );
      return Ok(());
    }

    let mut res = self.client.get(url).send().await?.error_for_status()?;

    let tmp = dest.with_extension("tmp");
    let mut file = fs::File::create(&tmp).await?;
    let mut hasher = Sha256::new();

    let total = res.content_length().unwrap_or(0);
    let mut received: u64 = 0;
    let mut last_report: Option<Instant> = None;

    while let Some(chunk) = res.chunk().await? {
      received += chunk.len() as u64;
      hasher.update(&chunk);
      file.write_all(&chunk).await?;
      // A report per chunk is thousands of events for ffmpeg: a few per second are enough.
      let finished = total > 0 && received >= total;
      if !finished && last_report.is_some_and(|at| at.elapsed() < PROGRESS_INTERVAL) {
        continue;
      }
      last_report = Some(Instant::now());
      let _ = self.app.emit(
        "binary_download_progress",
        ToolProgress {
          tool: tool.to_string(),
          received,
          total,
          reused: false,
        },
      );
    }

    let hash = hex::encode(hasher.finalize());
    if hash != sha256 {
      return Err("sha256 mismatch".into());
    }

    fs::rename(&tmp, dest).await?;
    Ok(())
  }

  async fn install_from_download(
    &self,
    tool: &str,
    dest: &Path,
    entry: Option<&str>,
    bundle: Option<&BundleInfo>,
    shared: &SharedArchive,
  ) -> Result<Vec<PathBuf>, AnyError> {
    let keep_archive = shared.keep_archive;
    let canonical = self.canonical_path(tool)?;
    // Binaries of other tools that were extracted from the same archive.
    let mut extras: Vec<PathBuf> = Vec::new();
    let parent = dest.parent().unwrap();

    if let Some(ext) = dest.extension().and_then(|e| e.to_str()) {
      match ext {
        "zip" => {
          if let Some(b) = bundle {
            let (dir, _entry_abs) = extract_zip_bundle(
              dest,
              parent,
              b.folder_name.as_deref(),
              Path::new(&b.entry),
              b.rename_entry_to.as_deref(),
            )
            .await?;
            remove_archive(dest, keep_archive).await?;
            self
              .hoist_bundle_contents_into_bin(&dir, &canonical)
              .await?;
          } else {
            extract_zip(tool, canonical.clone(), dest, parent, entry).await?;
            remove_archive(dest, keep_archive).await?;
          }
        }
        "bz2" | "xz" => {
          let compression = TarCompression::from_extension(ext).expect("bz2 or xz");
          if let Some(b) = bundle {
            let (dir, _entry_abs) = extract_tar_bundle(
              dest,
              parent,
              b.folder_name.as_deref(),
              Path::new(&b.entry),
              b.rename_entry_to.as_deref(),
              compression,
            )
            .await?;
            remove_archive(dest, keep_archive).await?;
            self
              .hoist_bundle_contents_into_bin(&dir, &canonical)
              .await?;
          } else if let Some(entry) = entry {
            // One pass over the archive for this tool and the later ones.
            let mut targets = vec![(canonical.clone(), entry.to_string())];
            targets.extend(
              shared
                .later_tools
                .iter()
                .map(|(_, path, entry)| (path.clone(), entry.clone())),
            );
            let written = extract_tar_entries(dest, parent, targets, compression).await?;
            extras = written.into_iter().filter(|p| *p != canonical).collect();
            remove_archive(dest, keep_archive).await?;
          } else {
            extract_tar(tool, canonical.clone(), dest, parent, None, compression).await?;
            remove_archive(dest, keep_archive).await?;
          }
        }
        _ => {
          fs::rename(dest, &canonical).await?;
        }
      }
    } else {
      fs::rename(dest, &canonical).await?;
    }

    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      for path in std::iter::once(&canonical).chain(extras.iter()) {
        let mut perms = std::fs::metadata(path)?.permissions();
        if perms.mode() & 0o111 == 0 {
          perms.set_mode(0o755);
          std::fs::set_permissions(path, perms)?;
        }
      }
    }

    Ok(extras)
  }

  async fn load_metadata(&self, path: &Path) -> Result<Metadata, AnyError> {
    if let Ok(bytes) = fs::read(path).await {
      let meta = serde_json::from_slice(&bytes)?;
      Ok(meta)
    } else {
      Ok(Metadata::default())
    }
  }

  async fn save_metadata(&self, path: &Path, meta: &Metadata) -> Result<(), AnyError> {
    let bytes = serde_json::to_vec(meta)?;
    fs::write(path, bytes).await?;
    Ok(())
  }

  fn emit_tool_error(&self, tool: &str, version: &str, stage: &str, error: impl Into<String>) {
    let _ = self.app.emit(
      "binary_download_error",
      ToolError {
        tool: tool.to_string(),
        version: version.to_string(),
        stage: stage.to_string(),
        error: error.into(),
      },
    );
  }

  pub async fn hoist_bundle_contents_into_bin(
    &self,
    bundle_root: &Path,
    canonical: &Path,
  ) -> Result<(), AnyError> {
    let bin_dir = canonical
      .parent()
      .ok_or("canonical has no parent directory")?
      .to_path_buf();

    tokio::fs::create_dir_all(&bin_dir).await?;

    let src = bundle_root.to_path_buf();
    tokio::task::spawn_blocking(move || replace_entries(&src, &bin_dir)).await??;

    let _ = fs::remove_dir_all(bundle_root).await;

    Ok(())
  }
}

/// Moves everything in `from` into `into`, replacing what is there (yt-dlp and
/// its `_internal` folder). The old entries are moved aside first and put back
/// if anything fails: on Windows the files of a running yt-dlp cannot be moved,
/// and copying over them would leave a mix of two versions.
fn replace_entries(from: &Path, into: &Path) -> std::io::Result<()> {
  remove_old_entries(into);
  let suffix = uuid::Uuid::new_v4();
  let mut moved_aside: Vec<(PathBuf, PathBuf)> = Vec::new();
  let mut placed: Vec<PathBuf> = Vec::new();

  let result = (|| -> std::io::Result<()> {
    for entry in std::fs::read_dir(from)? {
      let entry = entry?;
      let target = into.join(entry.file_name());
      if target.symlink_metadata().is_ok() {
        let backup = into.join(format!(
          "{OLD_PREFIX}{}-{suffix}",
          entry.file_name().to_string_lossy()
        ));
        std::fs::rename(&target, &backup)?;
        moved_aside.push((target.clone(), backup));
      }
      std::fs::rename(entry.path(), &target)?;
      placed.push(target);
    }
    Ok(())
  })();

  if let Err(err) = result {
    for target in placed.iter().rev() {
      remove_entry(target);
    }
    for (original, backup) in moved_aside.iter().rev() {
      let _ = std::fs::rename(backup, original);
    }
    return Err(err);
  }
  remove_old_entries(into);
  Ok(())
}

const OLD_PREFIX: &str = ".old-";

/// Removes what earlier updates moved aside. On Windows a file can stay in use
/// for a while, so this is retried on every update.
fn remove_old_entries(dir: &Path) {
  let Ok(entries) = std::fs::read_dir(dir) else {
    return;
  };
  for entry in entries.flatten() {
    if entry.file_name().to_string_lossy().starts_with(OLD_PREFIX) {
      remove_entry(&entry.path());
    }
  }
}

fn remove_entry(path: &Path) {
  if path.is_dir() && !path.is_symlink() {
    let _ = std::fs::remove_dir_all(path);
  } else {
    let _ = std::fs::remove_file(path);
  }
}

async fn remove_archive(archive: &Path, keep: bool) -> std::io::Result<()> {
  if keep {
    Ok(())
  } else {
    fs::remove_file(archive).await
  }
}

/// Whether `path` starts like a program of this system: ELF on Linux, PE (`MZ`) on
/// Windows. Scripts (`#!`) count too.
pub(crate) async fn is_program(path: &Path) -> bool {
  use tokio::io::AsyncReadExt;
  let Ok(mut file) = fs::File::open(path).await else {
    return false;
  };
  let mut magic = [0u8; 4];
  if file.read_exact(&mut magic).await.is_err() {
    return false;
  }
  looks_like_program(&magic)
}

fn looks_like_program(magic: &[u8; 4]) -> bool {
  if magic.starts_with(b"#!") {
    return true;
  }
  if cfg!(windows) {
    magic.starts_with(b"MZ")
  } else {
    magic == b"\x7fELF"
  }
}

/// The SHA-256 of a file, or `None` when it cannot be read (e.g. it does not exist).
async fn file_sha256(path: &Path) -> Option<String> {
  use tokio::io::AsyncReadExt;
  let mut file = fs::File::open(path).await.ok()?;
  let mut hasher = Sha256::new();
  let mut buf = vec![0u8; 1 << 20];
  loop {
    let read = file.read(&mut buf).await.ok()?;
    if read == 0 {
      break;
    }
    hasher.update(&buf[..read]);
  }
  Some(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
  use super::{
    BinariesManager, FileInfo, Manifest, OLD_PREFIX, is_program, looks_like_program,
    replace_entries, tools_for_channel,
  };
  use crate::state::config_models::YtDlpChannel;
  use std::collections::HashMap;

  fn manifest(with_nightly: bool) -> Manifest {
    let channels = if with_nightly {
      r#","channels":{"nightly":{"tools":{"yt-dlp":{"version":"2026.10.10.233141","files":{}}}}}"#
    } else {
      ""
    };
    serde_json::from_str(&format!(
      r#"{{"generatedAt":"x","tools":{{
        "yt-dlp":{{"version":"2026.08.19","files":{{}}}},
        "ffmpeg":{{"version":"2026-10","files":{{}}}}
      }}{channels}}}"#
    ))
    .unwrap()
  }

  fn versions(manifest: &Manifest, channel: YtDlpChannel) -> Vec<(String, String, bool)> {
    tools_for_channel(manifest, channel)
      .into_iter()
      .map(|t| (t.name.to_string(), t.info.version.clone(), t.keep_installed))
      .collect()
  }

  #[test]
  fn the_nightly_channel_replaces_only_yt_dlp() {
    let m = manifest(true);
    assert_eq!(
      versions(&m, YtDlpChannel::Nightly),
      [
        ("yt-dlp".into(), "2026.10.10.233141".into(), false),
        ("ffmpeg".into(), "2026-10".into(), false),
      ]
    );
    assert_eq!(
      versions(&m, YtDlpChannel::Stable)[0],
      ("yt-dlp".into(), "2026.08.19".into(), false)
    );
  }

  #[test]
  fn without_nightly_builds_the_installed_yt_dlp_is_kept() {
    // The manifest of a day when the nightly builds could not be fetched, or
    // one written before channels existed.
    let m = manifest(false);
    assert_eq!(
      versions(&m, YtDlpChannel::Nightly),
      [
        ("yt-dlp".into(), "2026.08.19".into(), true),
        ("ffmpeg".into(), "2026-10".into(), false),
      ]
    );
    assert!(!versions(&m, YtDlpChannel::Stable)[0].2);
  }

  fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ovd-replace-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn a_bundle_replaces_the_old_one_completely() {
    let dir = temp_dir();
    let bin = dir.join("bin");
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bin.join("_internal")).unwrap();
    std::fs::write(bin.join("yt-dlp"), "old").unwrap();
    std::fs::write(bin.join("_internal/old-only.so"), "old").unwrap();
    std::fs::write(bin.join("ffmpeg"), "keep").unwrap();
    std::fs::write(bin.join(format!("{OLD_PREFIX}yt-dlp-stale")), "stale").unwrap();
    std::fs::create_dir_all(bundle.join("_internal")).unwrap();
    std::fs::write(bundle.join("yt-dlp"), "new").unwrap();
    std::fs::write(bundle.join("_internal/new.so"), "new").unwrap();

    replace_entries(&bundle, &bin).unwrap();

    assert_eq!(std::fs::read_to_string(bin.join("yt-dlp")).unwrap(), "new");
    assert!(bin.join("_internal/new.so").exists());
    // Files of the old version do not stay behind.
    assert!(!bin.join("_internal/old-only.so").exists());
    assert_eq!(std::fs::read_to_string(bin.join("ffmpeg")).unwrap(), "keep");
    let mut names: Vec<_> = std::fs::read_dir(&bin)
      .unwrap()
      .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
      .collect();
    names.sort();
    assert_eq!(names, ["_internal", "ffmpeg", "yt-dlp"]);
    std::fs::remove_dir_all(dir).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn a_failed_replacement_puts_the_old_bundle_back() {
    use std::os::unix::fs::PermissionsExt;
    // Root can write to read-only folders, so the failure cannot be simulated.
    if unsafe { libc::geteuid() } == 0 {
      return;
    }
    let dir = temp_dir();
    let bin = dir.join("bin");
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bin.join("_internal")).unwrap();
    std::fs::write(bin.join("yt-dlp"), "old").unwrap();
    std::fs::write(bin.join("_internal/lib.so"), "old").unwrap();
    std::fs::create_dir_all(bundle.join("_internal")).unwrap();
    std::fs::write(bundle.join("yt-dlp"), "new").unwrap();
    std::fs::write(bundle.join("_internal/lib.so"), "new").unwrap();
    // A folder that cannot be moved, like a folder in use on Windows: its
    // parent is read-only, so the new entry cannot be moved out of the bundle.
    std::fs::set_permissions(&bundle, std::fs::Permissions::from_mode(0o555)).unwrap();

    let result = replace_entries(&bundle, &bin);
    std::fs::set_permissions(&bundle, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(bin.join("yt-dlp")).unwrap(), "old");
    assert_eq!(
      std::fs::read_to_string(bin.join("_internal/lib.so")).unwrap(),
      "old"
    );
    std::fs::remove_dir_all(dir).unwrap();
  }

  fn file(url: &str) -> FileInfo {
    FileInfo {
      url: url.into(),
      sha256: String::new(),
      entry: None,
      bundle: None,
    }
  }

  #[test]
  fn an_archive_is_not_a_program() {
    // The start of an .xz archive, installed as ffmpeg by versions that could not unpack it.
    assert!(!looks_like_program(&[0xfd, b'7', b'z', b'X']));
    assert!(!looks_like_program(&[0, 0, 0, 0]));
    assert!(looks_like_program(b"#!/b"));
  }

  #[cfg(not(windows))]
  #[test]
  fn an_elf_file_is_a_program() {
    assert!(looks_like_program(b"\x7fELF"));
    assert!(!looks_like_program(b"MZ\x90\0"));
  }

  #[cfg(windows)]
  #[test]
  fn a_pe_file_is_a_program() {
    assert!(looks_like_program(b"MZ\x90\0"));
    assert!(!looks_like_program(b"\x7fELF"));
  }

  #[tokio::test]
  async fn a_missing_or_short_file_is_not_a_program() {
    let dir = std::env::temp_dir().join(format!("ovd-program-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(!is_program(&dir.join("nothing")).await);
    std::fs::write(dir.join("short"), b"ab").unwrap();
    assert!(!is_program(&dir.join("short")).await);
    std::fs::write(dir.join("archive"), [0xfd, b'7', b'z', b'X', b'Z', 0]).unwrap();
    assert!(!is_program(&dir.join("archive")).await);
    std::fs::remove_dir_all(dir).unwrap();
  }

  #[test]
  fn the_file_of_the_platform_is_used() {
    let files = HashMap::from([
      ("windows-x86_64".to_string(), file("tool.zip")),
      ("linux-x86_64".to_string(), file("tool.tar.xz")),
    ]);
    let (key, chosen) = BinariesManager::select_file(&files, "linux-x86_64").unwrap();
    assert_eq!(key, "linux-x86_64");
    assert_eq!(chosen.url, "tool.tar.xz");
  }

  #[test]
  fn another_architecture_is_not_used() {
    let files = HashMap::from([
      ("linux-aarch64".to_string(), file("arm.bin")),
      ("linux-x86_64".to_string(), file("x86.bin")),
    ]);
    let (key, _) = BinariesManager::select_file(&files, "linux-aarch64").unwrap();
    assert_eq!(key, "linux-aarch64");
  }

  #[test]
  fn windows_on_arm_falls_back_to_the_x64_file() {
    let files = HashMap::from([
      ("windows-x86_64".to_string(), file("x64.zip")),
      ("linux-x86_64".to_string(), file("linux.zip")),
    ]);
    let (key, _) = BinariesManager::select_file(&files, "windows-aarch64").unwrap();
    assert_eq!(key, "windows-x86_64");
  }

  #[test]
  fn linux_on_arm_does_not_use_an_x64_file() {
    let files = HashMap::from([
      ("linux-x86_64".to_string(), file("x64.zip")),
      ("windows-x86_64".to_string(), file("win.zip")),
    ]);
    assert!(BinariesManager::select_file(&files, "linux-aarch64").is_none());
  }
}
