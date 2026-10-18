// Test videos made with ffmpeg and served from this machine, so the tests do not
// depend on any website.
import { execFileSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, statSync } from 'node:fs';
import { createServer, type Server } from 'node:http';
import { join } from 'node:path';

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
  missingUrl: string;
  close: () => Promise<void>;
}

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
    const size = statSync(file).size;
    const range = /bytes=(\d*)-(\d*)/.exec(req.headers.range ?? '');
    if (range) {
      const start = range[1] ? Number(range[1]) : 0;
      const end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
      res.writeHead(206, {
        'content-type': 'video/mp4',
        'content-length': end - start + 1,
        'content-range': `bytes ${start}-${end}/${size}`,
        'accept-ranges': 'bytes',
      });
      if (req.method === 'HEAD') res.end();
      else createReadStream(file, { start, end }).pipe(res);
      return;
    }
    res.writeHead(200, { 'content-type': 'video/mp4', 'content-length': size, 'accept-ranges': 'bytes' });
    if (req.method === 'HEAD') res.end();
    else createReadStream(file).pipe(res);
  });
  await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  const port = address && typeof address === 'object' ? address.port : 0;
  const base = `http://127.0.0.1:${port}/media`;
  return {
    url: name => `${base}/${name}.mp4`,
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
