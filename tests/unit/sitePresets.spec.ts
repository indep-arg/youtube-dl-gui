import { describe, expect, it } from 'vitest';
import { findSitePreset, normalizeDomain } from '../../src/helpers/sitePresets';
import type { SitePreset } from '../../src/tauri/types/config';
import { TrackType } from '../../src/tauri/types/media';

const preset = (domain: string, trackType: TrackType | null = null): SitePreset => ({
  domain,
  trackType,
  downloadDir: null,
});

describe('normalizeDomain', () => {
  it.each([
    ['youtube.com', 'youtube.com'],
    ['  YouTube.com ', 'youtube.com'],
    ['https://www.youtube.com/watch?v=1', 'youtube.com'],
    ['http://music.youtube.com:8080/', 'music.youtube.com'],
    ['www.example.org.', 'example.org'],
    ['', ''],
  ])('%s -> %s', (input, expected) => {
    expect(normalizeDomain(input)).toBe(expected);
  });
});

describe('findSitePreset', () => {
  const presets = [preset('youtube.com', TrackType.video), preset('music.youtube.com', TrackType.audio)];

  it('matches the site and its subdomains', () => {
    expect(findSitePreset('https://www.youtube.com/watch?v=1', presets)?.trackType).toBe(TrackType.video);
    expect(findSitePreset('https://m.youtube.com/watch?v=1', presets)?.trackType).toBe(TrackType.video);
  });

  it('prefers the most specific site', () => {
    expect(findSitePreset('https://music.youtube.com/watch?v=1', presets)?.trackType).toBe(TrackType.audio);
  });

  it('does not match a different site that only ends the same way', () => {
    expect(findSitePreset('https://notyoutube.com/v/1', presets)).toBeUndefined();
  });

  it('ignores presets without a site and links that are not URLs', () => {
    expect(findSitePreset('https://example.com/', [preset('')])).toBeUndefined();
    expect(findSitePreset('not a link', presets)).toBeUndefined();
  });

  it('accepts sites written as links', () => {
    expect(findSitePreset('https://vimeo.com/1', [preset('https://www.Vimeo.com/')])).toBeDefined();
  });
});
