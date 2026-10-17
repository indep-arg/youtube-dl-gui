use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::binaries::binaries_extractor::{
  extract_tar, extract_tar_bundle, extract_zip, extract_zip_bundle, TarCompression,
};
use crate::binaries::binaries_state::CheckResult;
use crate::paths::PathsManager;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fs_extra::dir::{move_dir, CopyOptions};
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

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolProgress {
  tool: String,
  total: u64,
  received: u64,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct ToolComplete {
  tool: String,
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
      client: reqwest::Client::new(),
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

  /// The file for this platform. A `-v2` key (e.g. `linux-x86_64-v2`) wins: it
  /// holds a build that only newer versions of the app install, while older
  /// versions keep using the plain key.
  ///
  /// Windows on ARM runs x64 programs, so a tool without an ARM build uses the x64
  /// one there. Linux on ARM cannot, so such a tool is left out.
  fn select_file<'a>(
    files: &'a HashMap<String, FileInfo>,
    arch: &'a str,
  ) -> Option<(&'a str, &'a FileInfo)> {
    if let Some((key, f)) = files.get_key_value(format!("{arch}-v2").as_str()) {
      return Some((key.as_str(), f));
    }
    if let Some(f) = files.get(arch) {
      return Some((arch, f));
    }
    if arch == "windows-aarch64" {
      return Self::select_file(files, "windows-x86_64");
    }
    None
  }

  /// The version recorded for an installed tool. A `-v2` file gets its own
  /// version, so a tool installed from the plain file (by an older version of the
  /// app) is replaced once by the `-v2` build, even if the tool version is the same.
  fn installed_version(info: &ToolInfo, key: &str) -> String {
    if key.ends_with("-v2") {
      format!("{}+v2", info.version)
    } else {
      info.version.clone()
    }
  }

  async fn build_plan<'a>(
    &self,
    manifest: &'a Manifest,
    meta: &Metadata,
    arch: &str,
    allow: Option<&[String]>,
  ) -> Result<Vec<(&'a str, &'a ToolInfo)>, AnyError> {
    let mut plan = Vec::new();
    for (name, info) in &manifest.tools {
      if let Some(allow) = allow {
        if !allow.iter().any(|n| n == name) {
          continue;
        }
      }

      let Some((key, _)) = Self::select_file(&info.files, arch) else {
        continue;
      };

      let version_ok = meta
        .versions
        .get(name)
        .is_some_and(|v| *v == Self::installed_version(info, key));
      let canonical = self.canonical_path(name)?;
      let exists = tokio::fs::metadata(&canonical).await.is_ok();

      if !version_ok || !exists {
        plan.push((name.as_str(), info));
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
    // downloaded once and kept until the last tool that needs it is installed.
    let urls: Vec<Option<String>> = plan
      .iter()
      .map(|(_, info)| Self::select_file(&info.files, &arch).map(|(_, f)| f.url.clone()))
      .collect();

    for (index, (name, info)) in plan.into_iter().enumerate() {
      let keep_archive = urls[index]
        .as_ref()
        .is_some_and(|url| urls[index + 1..].iter().flatten().any(|later| later == url));
      match self
        .install_single_tool(bin, &arch, name, info, keep_archive)
        .await
      {
        Ok(()) => {
          // only update meta + successes on fully successful install
          let key = Self::select_file(&info.files, &arch).map_or("", |(key, _)| key);
          meta
            .versions
            .insert(name.to_string(), Self::installed_version(info, key));
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
    keep_archive: bool,
  ) -> Result<(), ToolError> {
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

    if let Err(e) = self
      .download_and_verify(name, file, &dest, keep_archive)
      .await
    {
      return self.fail_stage(name, &info.version, "download_verify", e.to_string());
    }

    let canonical = self.canonical_path(name).map_err(|e| {
      self
        .fail_stage(name, &info.version, "canonical_path", e.to_string())
        .err()
        .unwrap()
    })?;

    if tokio::fs::metadata(&canonical).await.is_err() {
      let err = format!(
        "canonical binary missing after install: {}",
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

    Ok(())
  }

  fn fail_stage(
    &self,
    name: &str,
    version: &str,
    stage: &str,
    msg: String,
  ) -> Result<(), ToolError> {
    self.emit_tool_error(name, version, stage, &msg);
    Err(ToolError {
      tool: name.to_string(),
      version: version.to_string(),
      stage: stage.to_string(),
      error: msg,
    })
  }

  async fn fetch_manifest(&self) -> Result<Manifest, AnyError> {
    let manifest_bytes = self.client.get(MANIFEST_URL).send().await?.bytes().await?;

    let sig_b64 = self
      .client
      .get(MANIFEST_SIG_URL)
      .send()
      .await?
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

  async fn download_and_verify(
    &self,
    tool: &str,
    file: &FileInfo,
    dest: &Path,
    keep_archive: bool,
  ) -> Result<(), AnyError> {
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
        keep_archive,
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
        },
      );
      return Ok(());
    }

    let mut res = self.client.get(url).send().await?;

    let tmp = dest.with_extension("tmp");
    let mut file = fs::File::create(&tmp).await?;
    let mut hasher = Sha256::new();

    let total = res.content_length().unwrap_or(0);
    let mut received: u64 = 0;

    while let Some(chunk) = res.chunk().await? {
      received += chunk.len() as u64;
      hasher.update(&chunk);
      file.write_all(&chunk).await?;
      let _ = self.app.emit(
        "binary_download_progress",
        ToolProgress {
          tool: tool.to_string(),
          received,
          total,
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
    keep_archive: bool,
  ) -> Result<(), AnyError> {
    let canonical = self.canonical_path(tool)?;
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
          } else {
            extract_tar(tool, canonical.clone(), dest, parent, entry, compression).await?;
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
      let mut perms = std::fs::metadata(&canonical)?.permissions();
      if perms.mode() & 0o111 == 0 {
        perms.set_mode(0o755);
        std::fs::set_permissions(&canonical, perms)?;
      }
    }

    Ok(())
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
    let dest = bin_dir.clone();

    tokio::task::spawn_blocking(move || {
      let mut opts = CopyOptions::new();
      opts.overwrite = true;
      opts.copy_inside = false;
      opts.content_only = true;
      move_dir(&src, &dest, &opts)?;
      Ok::<(), fs_extra::error::Error>(())
    })
    .await??;

    let _ = fs::remove_dir_all(bundle_root).await;

    Ok(())
  }
}

async fn remove_archive(archive: &Path, keep: bool) -> std::io::Result<()> {
  if keep {
    Ok(())
  } else {
    fs::remove_file(archive).await
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
  use super::{BinariesManager, FileInfo, ToolInfo};
  use std::collections::HashMap;

  fn file(url: &str) -> FileInfo {
    FileInfo {
      url: url.into(),
      sha256: String::new(),
      entry: None,
      bundle: None,
    }
  }

  #[test]
  fn a_v2_file_wins_over_the_plain_one() {
    let files = HashMap::from([
      ("linux-x86_64".to_string(), file("old.tar.bz2")),
      ("linux-x86_64-v2".to_string(), file("new.tar.xz")),
    ]);
    let (key, chosen) = BinariesManager::select_file(&files, "linux-x86_64").unwrap();
    assert_eq!(key, "linux-x86_64-v2");
    assert_eq!(chosen.url, "new.tar.xz");
  }

  #[test]
  fn the_plain_file_is_used_without_a_v2_one() {
    let files = HashMap::from([("windows-x86_64".to_string(), file("tool.zip"))]);
    let (key, _) = BinariesManager::select_file(&files, "windows-x86_64").unwrap();
    assert_eq!(key, "windows-x86_64");
  }

  #[test]
  fn a_v2_file_has_its_own_installed_version() {
    let info = ToolInfo {
      version: "b6.1.1".into(),
      files: HashMap::new(),
    };
    assert_eq!(
      BinariesManager::installed_version(&info, "linux-x86_64-v2"),
      "b6.1.1+v2"
    );
    assert_eq!(
      BinariesManager::installed_version(&info, "linux-x86_64"),
      "b6.1.1"
    );
  }

  #[test]
  fn a_v2_file_of_another_architecture_is_not_used() {
    let files = HashMap::from([
      ("linux-aarch64".to_string(), file("arm.bin")),
      ("linux-x86_64-v2".to_string(), file("x86.tar.xz")),
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
