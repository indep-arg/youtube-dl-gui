export interface HistoryEntry {
  id: string;
  url: string;
  title: string | null;
  thumbnail: string | null;
  uploader: string | null;
  /** Where the (main) file was saved, when it is known. */
  path: string | null;
  /** How many files the download produced (more than one for a playlist). */
  items: number;
  /** Unix time in milliseconds. */
  finishedAt: number;
  isLive: boolean;
}

/** A link that was in the queue when the app was closed. */
export interface QueuedLink {
  url: string;
  overrides: unknown;
  options: unknown;
}

export interface Library {
  history: HistoryEntry[];
  queue: QueuedLink[];
}
