# Translating

The app has 13 languages. Texts live in JSON files.

Every language file has exactly the same keys as `en.json`, in the same order and with the same
structure. A text that is not translated yet keeps the English text, so the files can be compared
line by line with `en.json`.

There are two folders of translations:

- `src/locales/<code>.json`: the interface.
- `src-tauri/locales/<code>.json`: what the backend shows, like the tray menu and notifications.

`en.json` is the reference. The files are named after the language code (`es`, `pt-BR`, `zh-TW`…).

## Improving a language that already exists

1. Edit its file in both folders with any text editor.
2. Change only the text on the right-hand side of each key. Do not rename, add, remove or move keys.
3. Open a pull request.

When English gets a new key, add it to every language in the same place, with the English text until
someone translates it.

## Adding a new language

1. Copy `en.json` to `<code>.json` in both folders.
2. Translate the texts.
3. Register it in `src/i18n.ts`: the import, `availableLocales` and `messages`. A language that is not
   registered there cannot be selected, even if its file exists.

## Preview your changes

Run the app in development mode and edit the file; the interface updates when you save:

```bash
npm install
npm run tauri dev
```

To try a change in English, edit `src/locales/en.json` temporarily, and restore it before the pull request.

## Guidelines

- Keep the `{placeholders}` exactly as written (`{count}`, `{error}`, `{version}`, `{percent}`…): the app
  fills them in. In texts with plural forms (`one | other`), every form needs its own placeholders.
- Do not translate brand or technical names (Open Video Downloader, yt-dlp, ffmpeg, SponsorBlock, `.mp4`…).
- Keep any markup, like `<strong>text</strong>`, and translate only the visible text.
- Prefer short, clear labels to long sentences.

## Checks

```bash
npx vitest run tests/unit/translations.spec.ts
npm run lint
```

The test fails if a language does not have the same keys as English in the same order, or if a
`{placeholder}` was changed.
The linter checks the keys used by the interface.
