// Test videos made with ffmpeg and served from this machine, so the tests do not
// depend on any website.
import { execFileSync, spawn } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, readFileSync, statSync } from 'node:fs';
import { createServer, type Server } from 'node:http';
import { extname, join } from 'node:path';

const CLIPS = ['clip', 'second'] as const;

/** Makes 6 second videos with picture and sound, once. */
export function makeClips(ffmpeg: string, dir: string): void {
  mkdirSync(dir, { recursive: true });
  for (const [index, name] of CLIPS.entries()) {
    const file = join(dir, `${name}.mp4`);
    if (existsSync(file)) continue;
    execFileSync(ffmpeg, [
      '-v', 'error', '-y',
      '-f', 'lavfi', '-i', `testsrc=size=320x240:rate=25:duration=6`,
      '-f', 'lavfi', '-i', `sine=frequency=${440 + index * 220}:duration=6`,
      '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest',
      file,
    ]);
  }
}

export interface MediaServer {
  url: (name: string) => string;
  /** The link of any file in the folder, e.g. `live.m3u8`. */
  file: (name: string) => string;
  missingUrl: string;
  close: () => Promise<void>;
}

const CONTENT_TYPES: Record<string, string> = {
  '.mp4': 'video/mp4',
  '.m3u8': 'application/vnd.apple.mpegurl',
  '.ts': 'video/mp2t',
};

/** Serves the files of `dir` under /media/, with support for range requests. */
export async function serveMedia(dir: string): Promise<MediaServer> {
  const server: Server = createServer((req, res) => {
    const name = decodeURIComponent((req.url ?? '').replace(/^\/media\//, '').split('?')[0]);
    const file = join(dir, name);
    if (!req.url?.startsWith('/media/') || name.includes('/') || !existsSync(file)) {
      res.writeHead(404, { 'content-type': 'text/plain' });
      res.end('not found');
      return;
    }
    const type = CONTENT_TYPES[extname(name)] ?? 'application/octet-stream';
    const size = statSync(file).size;
    const range = /bytes=(\d*)-(\d*)/.exec(req.headers.range ?? '');
    if (range) {
      const start = range[1] ? Number(range[1]) : 0;
      const end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
      res.writeHead(206, {
        'content-type': type,
        'content-length': end - start + 1,
        'content-range': `bytes ${start}-${end}/${size}`,
        'accept-ranges': 'bytes',
      });
      if (req.method === 'HEAD') res.end();
      else createReadStream(file, { start, end }).pipe(res);
      return;
    }
    res.writeHead(200, { 'content-type': type, 'content-length': size, 'accept-ranges': 'bytes', 'cache-control': 'no-store' });
    if (req.method === 'HEAD') res.end();
    else createReadStream(file).pipe(res);
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  const port = address && typeof address === 'object' ? address.port : 0;
  const base = `http://127.0.0.1:${port}/media`;
  return {
    url: name => `${base}/${name}.mp4`,
    file: name => `${base}/${name}`,
    missingUrl: `${base}/missing.mp4`,
    close: () => new Promise(resolve => server.close(() => resolve())),
  };
}

interface ProbeResult {
  duration: number;
  streams: string[];
  codecs: string[];
}

export function probe(ffprobe: string, file: string): ProbeResult {
  const out = execFileSync(ffprobe, [
    '-v', 'error', '-show_entries', 'format=duration:stream=codec_type,codec_name', '-of', 'json', file,
  ]).toString();
  const json = JSON.parse(out) as { format: { duration: string }; streams: { codec_type: string; codec_name: string }[] };
  return {
    duration: Number(json.format.duration),
    streams: json.streams.map(s => s.codec_type),
    codecs: json.streams.map(s => s.codec_name),
  };
}

interface LiveStream {
  url: string;
  stop: () => Promise<void>;
}

/**
 * A live stream made by ffmpeg in real time: an HLS playlist without an end,
 * which yt-dlp records like any other live stream.
 */
export async function startLiveStream(ffmpeg: string, dir: string): Promise<LiveStream> {
  mkdirSync(dir, { recursive: true });
  const encoder = spawn(ffmpeg, [
    '-v', 'error', '-re',
    '-f', 'lavfi', '-i', 'testsrc=size=320x240:rate=25',
    '-f', 'lavfi', '-i', 'sine=frequency=440',
    '-c:v', 'libx264', '-preset', 'ultrafast', '-g', '25', '-pix_fmt', 'yuv420p', '-c:a', 'aac',
    '-f', 'hls', '-hls_time', '1', '-hls_list_size', '6', '-hls_flags', 'delete_segments',
    join(dir, 'live.m3u8'),
  ], { stdio: 'ignore' });
  const server = await serveMedia(dir);
  const stop = async () => {
    if (encoder.exitCode === null) {
      const exited = new Promise(resolve => encoder.once('exit', resolve));
      encoder.kill('SIGTERM');
      await exited;
    }
    await server.close();
  };

  // A few segments, so that yt-dlp sees a playlist that is going on.
  const playlist = join(dir, 'live.m3u8');
  const end = Date.now() + 20_000;
  while (!(existsSync(playlist) && (readFileSync(playlist, 'utf8').match(/\.ts/g) ?? []).length >= 3)) {
    if (Date.now() > end || encoder.exitCode !== null) {
      await stop();
      throw new Error('the test live stream did not start');
    }
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  return { url: server.file('live.m3u8'), stop };
}
