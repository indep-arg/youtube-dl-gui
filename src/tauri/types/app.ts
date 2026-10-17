export enum Platform {
  Windows = 'windows',
  Linux = 'linux',
  Unknown = 'unknown',
}

export enum NotificationKind {
  QueueAdded = 'queueAdded',
  QueueDownloading = 'queueDownloading',
  QueueFinished = 'queueFinished',
  VideoFinished = 'videoFinished',
  PlaylistFinished = 'playlistFinished',
  DownloadFailed = 'downloadFailed',
  VideoReady = 'videoReady',
  PlaylistReady = 'playlistReady',
}
