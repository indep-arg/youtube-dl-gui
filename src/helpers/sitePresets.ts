import type { SitePreset } from '../tauri/types/config.ts';

/** Turns what a user types (`https://www.YouTube.com/`, `youtube.com`) into a plain domain. */
export function normalizeDomain(input: string): string {
  let domain = input.trim().toLowerCase();
  domain = domain.replace(/^[a-z][a-z0-9+.-]*:\/\//, '');
  domain = domain.split(/[/?#]/)[0];
  domain = domain.replace(/:\d+$/, '');
  return domain.replace(/^www\./, '').replace(/\.$/, '');
}

/**
 * The preset for the site of `url`: the one with the longest domain that matches
 * the host or one of its parent domains, so `music.youtube.com` beats `youtube.com`.
 */
export function findSitePreset(url: string, presets: SitePreset[]): SitePreset | undefined {
  let host: string;
  try {
    host = normalizeDomain(new URL(url).hostname);
  } catch {
    return undefined;
  }

  let best: SitePreset | undefined;
  let bestLength = 0;
  for (const preset of presets) {
    const domain = normalizeDomain(preset.domain);
    if (!domain) continue;
    const matches = host === domain || host.endsWith(`.${domain}`);
    if (matches && domain.length > bestLength) {
      best = preset;
      bestLength = domain.length;
    }
  }
  return best;
}
