import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'fs';
import { join } from 'path';

type Messages = { [key: string]: string | Messages };

const FOLDERS = [
  { name: 'interface', dir: 'src/locales' },
  { name: 'tray and notifications', dir: 'src-tauri/locales' },
];

const readLocale = (dir: string, file: string): Messages => {
  return JSON.parse(readFileSync(join(dir, file), 'utf8')) as Messages;
};

// Flattens the messages into { 'a.b.c': 'text' }.
const flatten = (messages: Messages, prefix = ''): Record<string, string> => {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(messages)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'string') {
      out[path] = value;
    } else {
      Object.assign(out, flatten(value, path));
    }
  }
  return out;
};

// The names are compared as a set: plural forms ("one | few | many") repeat them,
// and languages with more plural forms than English repeat them more often.
const placeholders = (text: string): string[] => {
  return [...new Set([...text.matchAll(/\{(\w+)\}/g)].map(m => m[1]))].sort();
};

// Every language has the same keys as English, in the same order, so a file
// can be compared with en.json line by line. A text that is not translated yet
// keeps the English one. A changed {placeholder} breaks the text at runtime.
describe.each(FOLDERS)('translations ($name)', ({ dir }) => {
  const english = flatten(readLocale(dir, 'en.json'));
  const others = readdirSync(dir).filter(file => file.endsWith('.json') && file !== 'en.json');

  it('has translations besides English', () => {
    expect(others.length).toBeGreaterThan(0);
  });

  it.each(others)('%s has the same keys as English, in the same order', (file) => {
    expect(Object.keys(flatten(readLocale(dir, file)))).toEqual(Object.keys(english));
  });

  it.each(others)('%s keeps the placeholders of English', (file) => {
    const broken = Object.entries(flatten(readLocale(dir, file)))
      .filter(([key, text]) => key in english && placeholders(text).join() !== placeholders(english[key]).join())
      .map(([key]) => key);
    expect(broken).toEqual([]);
  });
});
