// Sets the app's version everywhere it's kept (TECH.md §11): package.json,
// the Cargo workspace (both crates inherit it) and tauri.conf.json, then lets
// Cargo update Cargo.lock. Run with `pnpm set-version 0.2.0`.
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

/** A plain SemVer version, with an optional pre-release such as `-rc.1`. */
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z.-]+)?$/;

/** The JSON files, and how each sets the version without reformatting the rest. */
const JSON_VERSION = /^(\s*"version":\s*")[^"]*(")/m;
/** `version` in the `[workspace.package]` table of the root Cargo.toml. */
const CARGO_VERSION = /(\[workspace\.package\][^[]*?^version\s*=\s*")[^"]*(")/m;

export const FILES = [
  { path: 'package.json', pattern: JSON_VERSION },
  { path: 'src-tauri/tauri.conf.json', pattern: JSON_VERSION },
  { path: 'Cargo.toml', pattern: CARGO_VERSION },
];

/**
 * `text` with the version that `pattern` finds replaced by `version`.
 * @param {string} text
 * @param {RegExp} pattern
 * @param {string} version
 */
export function setVersion(text, pattern, version) {
  if (!pattern.test(text)) throw new Error('no version found');
  return text.replace(pattern, `$1${version}$2`);
}

/** @param {string} version */
export function isVersion(version) {
  return SEMVER.test(version);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const version = process.argv[2]?.replace(/^v/, '');
  if (!version || !isVersion(version)) {
    console.error('Usage: pnpm set-version <major.minor.patch>');
    process.exit(1);
  }
  for (const { path, pattern } of FILES) {
    const file = join(ROOT, path);
    writeFileSync(file, setVersion(readFileSync(file, 'utf8'), pattern, version));
    console.log(`Set ${path} to ${version}`);
  }
  execFileSync('cargo', ['update', '--workspace', '--offline'], { cwd: ROOT, stdio: 'inherit' });
}
