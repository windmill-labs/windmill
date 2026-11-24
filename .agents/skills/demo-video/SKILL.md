---
name: demo-video
description: Record a demo video of a feature, either a narrated browser walkthrough (mp4 with burned-in captions and text-to-speech audio) driven by Playwright, or a terminal session (a CLI or TUI such as Codex or Claude Code) recorded with vhs, optionally beside the browser it drives, with its idle waits cut. Use when the user asks for a demo video, a recorded walkthrough, a terminal or CLI recording, a video to check a feature, a video for a PR, or to cut the waiting out of a recording.
---

# Demo video

One scenario file in, one mp4 out. Do not snapshot or screenshot the page while writing the
scenario: write selectors from the code you just changed, run once, and fix only the step the
script reports as failed.

Every script lives in `.agents/skills/demo-video/`; the commands below are written from the repo
root. Scratch files (scenarios, tapes, recordings) go in the session scratchpad, never in the
repo: `record.mjs` writes only the mp4 you name, beside the scenario by default.

For a terminal session (vhs, optionally beside a browser), for cutting the waits out of any
silent recording, and for adding a voice-over to a cut, follow
`.agents/skills/demo-video/terminal.md`. A browser take whose waits depend on an AI (a chat
turn) goes the same way: record it silent with Playwright's `recordVideo`, cut, then narrate,
rather than through `record.mjs`, whose narration cannot survive a cut.

## What it needs

| Tool | Used by | Without it |
| --- | --- | --- |
| Playwright: `npm ci --prefix .agents/skills/demo-video`, once per worktree | every script | the script stops with that command |
| Playwright's Chromium: `npx --prefix .agents/skills/demo-video playwright install chromium`, once per machine | every script | the script stops with that command |
| `ffmpeg` and `ffprobe` (`brew install ffmpeg`) | every script | the script stops before recording |
| macOS `say` | narration in `record.mjs` and `narrate.mjs` | set `voice: false` in the scenario or pass `--voice=none`: captions only, no audio, each line on screen for its reading time |
| `vhs` (`brew install vhs`, brings `ttyd`) | terminal recordings | no terminal recordings; browser demos still work |

Homebrew's `ffmpeg` has no text filters, so captions and notes are drawn by the browser and
overlaid as images.

## Write the scenario

`<scratch>/demo.mjs`, against this worktree's frontend (its URL is in the session's
instructions, or `FRONTEND_PORT` in `.env.local`):

```js
export default {
  baseURL: 'http://localhost:3000',
  start: '/?workspace=admins',         // first page shown in the video
  // Runs unrecorded; its cookies carry into the recording. Log in here.
  setup: async ({ page }) => {
    const r = await page.request.post('/api/auth/login',
      { data: { email: 'admin@windmill.dev', password: 'changeme' } })
    if (!r.ok()) throw new Error(`login: ${r.status()}`)
  },
  steps: [
    { say: 'One sentence about what the viewer sees.' },
    { say: 'Clicking the row opens it.', do: ({ click }) => click('text=Daily orders') },
    { say: 'Hover shows the menu.', do: async ({ page, hover }) => { await hover('[aria-label="Menu"]') } },
  ],
}
```

- `say`: narration, spoken and shown as the caption. It starts before the step's action, and
  the step stays on screen until it is spoken. Keep each to one short sentence.
- `do`: gets `{ page, click, hover, wait }`. `click`/`hover` draw a ring on the target first, so
  prefer them over `page.click`. Strings are Playwright selectors (the first match is used);
  a locator is used as given, so a locator matching two elements fails the step. Assert the
  outcome inside the step by waiting on what proves it worked, e.g.
  `await page.getByText('Hello, Ada!').waitFor({ timeout: 30000 })`, so a broken feature fails
  the run instead of recording a wrong screen. There is no `expect` in a scenario.
- Captions sit in a band of their own below the page (`captionAt: 'top'` puts it above), so
  they never cover the UI. The mp4 is the viewport plus that band: 1280x816 by default.
- Optional: `hold` (extra ms after a step), `viewport` (default 1280x720), `voice` (macOS
  `say -v '?'`, default Samantha; `false` for no speech), `settle` (ms after `start` loads),
  `pad` (ms after each line, default 400), `captionHeight` (band height, default 96, two lines
  of text; viewport height + band must be even for H.264).
- Text selectors (`text=…`) can match hidden copies; for menu items use a role locator,
  `({ page, hover }) => hover(page.getByRole('menuitem', { name: /Shared sessions/ }))`.
- Aim for 5 to 10 steps, under a minute: the video is for checking a feature, not marketing.

## Test data must match the story

Plant the data the take needs before recording, and make it what the narration and the prompt
say it is. An AI agent in the take reads that data back and reacts to it: given a "Supabase"
connection string that pointed at `127.0.0.1`, Codex refused to call it a Supabase connection
and the take was unusable. Hosts, names, providers and values must fit the story; when the demo
needs the thing to really work, ask the user for a real instance of that kind rather than
planting a local stand-in. Never put a real secret on screen.

## Run

```bash
node .agents/skills/demo-video/record.mjs <scratch>/demo.mjs [<scratch>/demo.mp4]
```

It prints the mp4 path, or `step N failed: …` plus a screenshot path. Read that screenshot
only on failure.

## Deliver

The user may not be on this machine, so a local path is not a delivery.

- **Artifact page**: publish the mp4 through the Artifact `files` map (15 MB per file) and
  reference it from a `<video controls>`. The captions are burned in, so nothing else is
  needed. For a video made with `--no-band`, Artifacts do not serve `.vtt` files: write the
  cues with `narrate.mjs --cues=cues.json` and add them from inline JS:

  ```js
  const track = video.addTextTrack('captions', 'English', 'en')
  for (const [start, end, text] of CUES) track.addCue(new VTTCue(start, end, text))
  track.mode = 'showing'
  ```

- **PR body**: GitHub strips `<video>` from PR bodies and plays no mp4 from a raw URL (it
  downloads), so embed a GIF and link the narrated mp4. Make the GIF (about 4 MB for 55 s):

  ```bash
  ffmpeg -v error -y -i demo.mp4 -vf "fps=10,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=diff_mode=rectangle" demo.gif
  ```

  Upload both to `windmill-labs/agent-screenshots-internal`, as the `pr` skill does for
  screenshots. The repo is public and its history permanent: nothing secret may appear in the
  video.

  ```bash
  REPO=windmill-labs/agent-screenshots-internal
  for F in demo.gif demo.mp4; do
    DEST="shots/$(git branch --show-current)/$(date +%s)-$(basename "$F")"
    base64 < "$F" | tr -d '\n' | jq -Rs --arg m "add $DEST" '{message:$m, content:.}' \
      | gh api -X PUT "repos/$REPO/contents/$DEST" --input - >/dev/null
    URL="https://raw.githubusercontent.com/$REPO/main/$DEST"
    echo "$URL $(curl -sI "$URL" | grep -i '^content-length')"
  done
  ```

  Check that each `content-length` matches the local file, then put
  `![demo](<gif url>)` and `[Narrated mp4](<mp4 url>)` in the body.
