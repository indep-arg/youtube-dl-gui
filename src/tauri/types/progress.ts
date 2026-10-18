export interface MediaProgressStagePayload {
  id: string;
  groupId: string;
  stage: ProgressStage;
}

export interface MediaProgressPayload {
  id: string;
  groupId: string;
  category: ProgressCategory;
  percentage?: number;
  speedBps?: number;
  etaSecs?: number;
  /** Live streams: bytes recorded so far. */
  downloadedBytes?: number;
  /** Live streams: how much of the stream is recorded, when known. */
  recordedSecs?: number;
}

export interface MediaGroupProgressPayload extends MediaItemsProgress {
  id: string;
  groupId: string;
  speedBps?: number;
}

export interface MediaItemsProgress {
  total?: number;
  ready?: number;
  done?: number;
  /** Items that ended with an error. They are counted in `done` too. */
  failed?: number;
  downloading?: number;
}

export interface MediaProgressCompletePayload {
  id: string;
  groupId: string;
}

export enum ProgressCategory {
  video = 'video',
  audio = 'audio',
  subtitles = 'subtitles',
  thumbnail = 'thumbnail',
  metadata = 'metadata',
  other = 'other',
}

export enum ProgressStage {
  initializing = 'initializing',
  downloading = 'downloading',
  merging = 'merging',
  remuxing = 'remuxing',
  reencoding = 'reencoding',
  finalizing = 'finalizing',
}
