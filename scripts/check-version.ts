import { readFile } from 'fs/promises';

// The version must be the same in package.json, src-tauri/Cargo.toml and
// src-tauri/tauri.conf.json, and follow the YEAR.MONTH.N scheme (the month has
// no leading zero, since semver does not allow one): 2026.10.0
const CALVER = /^\d{4}\.(?:[1-9]|1[0-2])\.(?:0|[1-9]\d*)$/;

async function main() {
  const pkg = JSON.parse(await readFile('package.json', 'utf8')) as { version?: string };
  const conf = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8')) as { version?: string };
  const cargo = await readFile('src-tauri/Cargo.toml', 'utf8');
  const cargoVersion = /^\[package\][^[]*?^version\s*=\s*"([^"]+)"/ms.exec(cargo)?.[1];

  const versions: Record<string, string | undefined> = {
    'package.json': pkg.version,
    'src-tauri/Cargo.toml': cargoVersion,
    'src-tauri/tauri.conf.json': conf.version,
  };
  for (const [file, version] of Object.entries(versions)) {
    console.log(`[check-version] ${file}: ${version ?? 'missing'}`);
  }

  const distinct = new Set(Object.values(versions));
  if (distinct.size !== 1) {
    throw new Error('The version is not the same in all files');
  }
  const [version] = [...distinct];
  if (!version || !CALVER.test(version)) {
    throw new Error(`"${version}" does not follow the YEAR.MONTH.N scheme (e.g. 2026.10.0)`);
  }
  console.log(`[check-version] OK: ${version}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
