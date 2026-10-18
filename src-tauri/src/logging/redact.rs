//! Hides the IP address that some sites put in their media links (YouTube:
//! `…/ip/203.0.113.7/…` or `…&ip=203.0.113.7&…`), so that logs pasted into a
//! bug report do not publish it.

use std::borrow::Cow;

const HIDDEN: &str = "hidden";

pub fn hide_addresses(line: &str) -> Cow<'_, str> {
  if !line.contains("http") || !(line.contains("/ip/") || line.contains("ip=")) {
    return Cow::Borrowed(line);
  }

  let mut out = String::with_capacity(line.len());
  let mut rest = line;
  while let Some(start) = find_url(rest) {
    out.push_str(&rest[..start]);
    let from_url = &rest[start..];
    let end = from_url
      .find(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '<' | '>'))
      .unwrap_or(from_url.len());
    out.push_str(&hide_in_url(&from_url[..end]));
    rest = &from_url[end..];
  }
  out.push_str(rest);
  Cow::Owned(out)
}

fn find_url(text: &str) -> Option<usize> {
  [text.find("http://"), text.find("https://")]
    .into_iter()
    .flatten()
    .min()
}

fn hide_in_url(url: &str) -> String {
  let mut out = String::with_capacity(url.len());
  let mut rest = url;
  loop {
    let starts = ["/ip/", "?ip=", "&ip="]
      .iter()
      .filter_map(|marker| rest.find(marker).map(|i| i + marker.len()))
      .min();
    let Some(value_start) = starts else {
      break;
    };
    let value_len = rest[value_start..]
      .find(['/', '&', '?', '#'])
      .unwrap_or(rest.len() - value_start);
    out.push_str(&rest[..value_start]);
    out.push_str(if value_len == 0 { "" } else { HIDDEN });
    rest = &rest[value_start + value_len..];
  }
  out.push_str(rest);
  out
}

#[cfg(test)]
mod tests {
  use super::hide_addresses;

  #[test]
  fn hides_the_address_in_a_youtube_link() {
    let line = "[in#0 @ 0x55] Opening 'https://rr2---sn-abc.googlevideo.com/videoplayback/id/x.5/ip/190.136.75.141/requiressl/yes/file/seg.ts' for reading";
    assert_eq!(
      hide_addresses(line),
      "[in#0 @ 0x55] Opening 'https://rr2---sn-abc.googlevideo.com/videoplayback/id/x.5/ip/hidden/requiressl/yes/file/seg.ts' for reading"
    );
  }

  #[test]
  fn hides_the_address_in_a_query() {
    assert_eq!(
      hide_addresses("ERROR: https://a.example/v?id=1&ip=2001:db8::1&sig=x failed"),
      "ERROR: https://a.example/v?id=1&ip=hidden&sig=x failed"
    );
    assert_eq!(
      hide_addresses("https://a.example/v?ip=10.0.0.1"),
      "https://a.example/v?ip=hidden"
    );
  }

  #[test]
  fn leaves_other_lines_alone() {
    let path = "[download] Destination: /home/me/ip/clip.mp4";
    assert_eq!(hide_addresses(path), path);
    let link = "[generic] Extracting URL: https://example.com/zip=1&tip=2";
    assert_eq!(hide_addresses(link), link);
  }
}
