use crate::state::config_models::PerformanceSettings;

/// yt-dlp retries a download, and each fragment of it, 10 times by default.
const DEFAULT_RETRIES: u32 = 10;

pub fn build_performance_args(performance: &PerformanceSettings) -> Vec<String> {
  let mut args = Vec::new();

  if let Some(limit) = performance.rate_limit_kib.filter(|limit| *limit > 0) {
    args.push("--limit-rate".into());
    args.push(format!("{limit}K"));
  }

  if performance.retries != DEFAULT_RETRIES {
    let retries = performance.retries.to_string();
    args.push("--retries".into());
    args.push(retries.clone());
    args.push("--fragment-retries".into());
    args.push(retries);
  }

  if performance.concurrent_fragments > 1 {
    args.push("--concurrent-fragments".into());
    args.push(performance.concurrent_fragments.to_string());
  }

  args
}
