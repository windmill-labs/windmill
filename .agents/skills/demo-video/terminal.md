# Terminal recordings

A CLI or TUI session recorded with vhs, optionally beside a browser that reacts to it, then its
waits cut and a voice-over added. What each tool needs is in `SKILL.md` next to this file.

The take runs from a folder of the session scratchpad: the tape, the browser script, `vhs` and
the agent all start there, so the files they write (mp4, webm, markers, screenshots) never land
in a repo. The skill's scripts are called by their path,
`node <repo>/.agents/skills/demo-video/<script>.mjs`, from wherever you are.

Order of work: plant the test data (it must match the story, see `SKILL.md`), record, compose,
measure with `activity.mjs`, cut, measure the cut, narrate.

## Record the terminal (vhs)

`brew install vhs` (brings ttyd). Tape skeleton:

```
Output term.mp4
Set Shell zsh
Set FontSize 15
Set Width 820
Set Height 820
Set Padding 16
Set TypingSpeed 35ms
Hide
Type "export MY_TOKEN=$(cat <scratch>/token); clear"
Enter
Show
Type "mycli"
Enter
Wait+Screen@150s /text that stays once it is done/
Sleep 3s
```

- Run `vhs` from the tape's folder with a relative `Output`: an absolute path fails to parse
  (`Expected file path after output`).
- Everything between `Hide` and `Show` runs off camera: logins, tokens, aliases. Never type a
  secret after `Show`.
- `Wait+Screen@150s /regex/` waits until the regex matches anywhere on the screen, up to 150 s.
  Plain `Wait /regex/` reads only the last line; without `@<time>` the timeout is
  `Set WaitTimeout`. The typed command is on the screen too, so wait on text it does not
  contain (`echo "build $(echo finished)"`, then wait on `/build finished/`). A timed-out wait
  ends the run with only `recording failed` and no mp4.
- Once a plain shell's output scrolls, vhs 0.12's `Wait+Screen` no longer sees the bottom rows:
  `wmill script run` printed its result below the first screenful and a wait on it timed out
  while the text was on screen. Make the terminal tall enough that the take never scrolls
  (`Set Height`), or `clear` off camera before the command. Codex renders in the normal
  buffer and scrolls too: for a long agent turn, a generous `Sleep` is safer than a wait, and
  the cut removes the idle time. To see what a failing wait missed, replace it with `Sleep` +
  `Screenshot f.png`.
- A CLI that may open with a dialog (Codex's trust prompt, its "Background server has
  incompatible feature set" prompt after another Codex run changed its settings) is started in
  the `Hide` block and the dialog answered there (`Type "1"` picks "Run without daemon this
  time"), so the take opens on the ready prompt.
- Wait on text that stays (a question, the final answer), not on a spinner or a status that
  flips back. `Sleep` only for what the tape cannot see, such as another process acting; the
  cut step removes idle time afterwards, so err long.
- Keep long command lines off camera: define a function in the `Hide` block
  (`Type "mycli() { command mycli --flag … \"$@\"; }; clear"`) and type only `mycli` after
  `Show`. 500 characters at 35 ms is 17 s of typing that the cut step keeps.
- A Type string holding both quote kinds goes in backticks: ``Type `cmd -c 'a="b"'` ``.
- `Screenshot f.png` takes a still. macOS `date` has no `%3N`: take ms timestamps with
  `node -e 'console.log(Date.now())'`.

### The mp4 runs shorter than the session

vhs drops frames when the machine is busy, so its mp4 is shorter than the wall-clock session:
151.7 s of video for about 160 s of session in one Codex take, the gap growing over the take.
A wall-clock timestamp (an agent's log, a `Date.now()` marker) therefore points at the wrong
moment of the video, more so the later it is. Use wall clock only for a first guess at the
start of a take; place every cut, alignment and narration line from the picture, with
`activity.mjs` (below).

## Codex CLI, unattended

```
codex -a never -s read-only \
  -c check_for_update_on_startup=false \
  -c 'mcp_servers.<name>.url="https://<host>/<mcp path>"' \
  -c 'mcp_servers.<name>.bearer_token_env_var="MY_TOKEN"' \
  -c 'mcp_servers.<name>.default_tools_approval_mode="approve"'
```

- `-a never` never asks; without `default_tools_approval_mode="approve"` every MCP tool call is
  refused instead of run.
- The folder trust prompt cannot be skipped from `-c` (a `projects."<folder>".trust_level`
  override still shows it, in Codex 0.159). Accept it once for the take folder in a throwaway
  tape: `Type "codex"`, `Enter`, `Wait+Screen@30s /Trust and continue/`, `Enter`, `Sleep 5s`,
  `Ctrl+C`, `Ctrl+C`. The answer is saved in `~/.codex/config.toml` under the folder's real
  path (on macOS, `/private/tmp/…` for the scratchpad).
- `check_for_update_on_startup=false` keeps the update banner out of the shot.
- The token stays in an env var exported in the `Hide` block. Windmill's own MCP server is
  `<frontend URL>/api/mcp`, with a token that has the `mcp:all` scope.
- Codex writes each session to `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` as it goes.

## Claude Code, unattended

```
claude --strict-mcp-config --mcp-config <scratch>/mcp.json --allowedTools mcp__<name>
```

- `mcp.json` is `{"mcpServers": {"<name>": {"type": "http", "url": "…", "headers":
  {"Authorization": "Bearer …"}}}}`. It holds the token, so keep it in the scratchpad and never
  `cat` it on camera.
- `--allowedTools mcp__<name>` runs that server's tools without asking; anything else still
  asks, which a demo usually wants to show.
- The folder trust prompt is saved per folder: accept it once in a throwaway tape
  (`Type "claude"`, `Enter`, `Wait+Screen@30s /trust/`, `Enter`, `Sleep 5s`, `Ctrl+C`, `Ctrl+C`).
- Claude Code writes each session to `~/.claude/projects/<folder path, each non-alphanumeric character as ->/*.jsonl`.
- Its layout differs from Codex's: measure its regions from crops (below) rather than reusing
  the Codex example.

## Drive a browser beside it

A Playwright script that records its own video and waits for the agent's output:

```js
import { writeFileSync, existsSync } from 'node:fs'
import { launchBrowser, waitForOutput } from '<repo>/.agents/skills/demo-video/lib.mjs'

const since = Date.now()
writeFileSync('browser.pid', String(process.pid))
writeFileSync('browser-start.txt', String(since))
const browser = await launchBrowser()
const ctx = await browser.newContext({
  viewport: { width: 900, height: 820 },
  recordVideo: { dir: 'bvideo', size: { width: 900, height: 820 } }
})
const page = await ctx.newPage()
await page.request.post(`${BASE}/api/auth/login`, { data: { email: 'admin@windmill.dev', password: 'changeme' } })
await page.goto(`${BASE}/some/page`)
const [link] = await waitForOutput(/https:\/\/[^\s"\\]+\/the-page\?[^\s"\\]+/, { agent: 'codex', since })
await page.goto(link)
try {
  await page.getByText(/the success message/).waitFor({ timeout: 40000 })
} catch {
  await page.screenshot({ path: 'browser-fail.png' })
  console.log('end state not seen:', (await page.locator('body').innerText()).slice(0, 400))
}
while (!existsSync('done.txt')) await new Promise((r) => setTimeout(r, 1000))
await ctx.close()
await browser.close()
```

- `waitForOutput(pattern, { agent, cwd, since, timeout })` polls the logs of the `agent`
  (`'codex'` or `'claude'`) sessions started in `cwd` (default: the folder `node` was started
  from, so start the browser script from the folder vhs starts the agent in) and returns the
  match. Other agents on the machine write logs during the take too, a review in the same
  worktree or the recording agent itself, and only this folder's sessions count. It finds a log
  folder created during the take (Codex's per-day folder), so never build a dated path yourself.
  Logs are JSON: end the pattern before `"` and `\`. Take `since` when the script starts, before
  the agent can answer.
- `since` skips log files last written before it, but a file written after it is searched from
  the top. Start a new agent session for each take: a resumed session's log still holds the
  links of earlier takes, and the first match wins.
- The log holds the prompt and every tool result too, and the first match wins. Make the
  pattern match only the reply: tool results carry look-alike URLs (Windmill's deploy tool
  returns the page under the instance's base URL, not the host the prompt asked for), and the
  prompt must not contain the full link. Pin the host the prompt names and the path it leaves
  for the agent to fill in.
- Log in with `page.request.post(...)` before the first navigation.
- Run both, then a marker: `node browser.mjs & vhs term.tape; touch done.txt`. The script closes
  its context once `done.txt` exists, which finishes the webm.
- Before every take, stop the driver left by the previous one: `kill $(cat browser.pid)`. A
  stray one matches the new session's output and acts on it first. Never `pkill -f browser.mjs`,
  which also kills the takes of other worktrees.
- End the browser script on the state the page settles in (the success message), not a
  transient one, and keep the try/catch so a failed take says why.

## Compose side by side

```bash
ffmpeg -i term.mp4 -ss <offset> -i browser.webm -filter_complex \
  "[0:v]fps=25,scale=820:820,setsar=1[a];[1:v]fps=25,scale=900:820,setsar=1[b];[a][b]hstack=2,format=yuv420p[v]" \
  -map "[v]" -t <terminal duration> -c:v libx264 -crf 12 -preset veryfast side.mp4
```

- `<offset>`: seconds the browser recording began before the terminal one, trimmed off the
  browser input. Take a first guess from the start timestamps, then set it from frames: the
  page must open right after the link prints. Check the last interaction between the two sides
  as well; when the terminal has drifted (above) and the browser reacts visibly early, split
  the composition at that interaction or retime the browser with `setpts=<factor>*PTS`.
- Same height on both sides. crf 12 keeps text sharp for the cut step, which encodes again.

## Watch regions

`regions.json` lists the areas to watch, in pixels of the input. Example: a Codex TUI, 820x820
at font size 15, left of a 900-wide browser:

```json
[
  { "name": "transcript", "x": 0, "y": 0, "w": 820, "h": 670, "min": 150 },
  { "name": "status", "x": 0, "y": 670, "w": 820, "h": 35, "min": 30,
    "unlessLike": { "x": 40, "y": 675, "w": 72, "h": 22, "at": 40 } },
  { "name": "composer", "x": 0, "y": 705, "w": 820, "h": 65, "min": 300 },
  { "name": "browser", "x": 820, "y": 0, "w": 900, "h": 820, "min": 30 }
]
```

- Place the rows from two crops of the bottom of the terminal, one while the CLI works and one
  after: `ffmpeg -ss <t> -i side.mp4 -frames:v 1 -vf crop=820:180:0:640 f.png`.
- Codex's `Working (12s • esc to interrupt)` row ticks and shimmers, so watch it only
  `unlessLike` its look at a time (`at`) it shows "Working": when the work ends, the newest line
  of the answer lands in that same row.
- Composer: a cursor blink changes ~170 to 210 px per sample and typing 460 or more, so
  `min` 300. Leave the footer under it out.
- A region whose `min` is too low keeps the whole take "active" and nothing gets cut: run
  `activity.mjs` first and look for a run of equal small counts every ~0.6 s (the blink).

## See when the screen changes

```bash
node <repo>/.agents/skills/demo-video/activity.mjs side.mp4 regions.json [from] [to]
```

prints, per region, the runs of change in seconds of that file: `48.0-48.5(4744)` is a change
from 48.0 s to 48.5 s, 4744 pixels at most in one sample (`--raw` lists every sample). It
samples exactly as `cut-waits.mjs` does. Use it for every time you write down: where an agent's
work starts and ends (for `--cut=`), when the link prints and the page opens (for the offset),
and, on the cut file, where each narration line goes.

## Cut the waits

```bash
node <repo>/.agents/skills/demo-video/cut-waits.mjs side.mp4 out.mp4 regions.json --plan  # cuts + check sheets
node <repo>/.agents/skills/demo-video/cut-waits.mjs side.mp4 out.mp4 regions.json         # render
```

- Read every `<out>.cuts-N.png` row before rendering: the two frames of a row must show the
  same content, apart from timers and the cursor. Text appearing between them means a region
  missed it: fix the region and rerun `--plan`.
- Defaults: stretches of 3 s or more shrink to 1 s (0.7 s kept before the cut, 0.3 s after), 5 s
  kept after the last change, and a "⏩ N s later" note at the top centre for 1.4 s after each
  cut. Override with `--min-gap`, `--head`, `--tail`, `--end-hold`, `--label-y`.
- What changes stays: typing, tool calls appearing, links, the page being filled, the final
  answer. Time spent waiting on a tool is cut; the call and its result both stay on screen.
- An AI agent's own work is not idle to the measurement: its tool calls keep scrolling in while
  the viewer only waits for the answer. Remove those stretches with `--cut=<from>-<to>`
  (seconds of the input, repeatable), from just after the agent's first reply to just before the
  output the viewer needs (its message, the link, the final answer). Take both bounds from
  `activity.mjs` on `side.mp4`, never from the agent's log timestamps (the mp4 drifts from the
  wall clock), and confirm them on the check sheet. A forced cut gets the same note.
- Silent inputs only: the output carries no audio. Add the voice-over after cutting.

## Add a voice-over

```bash
node <repo>/.agents/skills/demo-video/narrate.mjs out.mp4 lines.json narrated.mp4 --plan --regions=out.regions.json
node <repo>/.agents/skills/demo-video/narrate.mjs out.mp4 lines.json narrated.mp4
```

lines.json: `[{ "at": 0.3, "text": "One sentence." }, ...]`, `at` in seconds of the cut video.
Take each `at` from `activity.mjs out.mp4 out.regions.json`: 0.3 s before the change the line
describes. `out.regions.json` is written by `cut-waits.mjs`: the same regions, with each
`unlessLike.at` moved onto the cut's timeline. If that `at` fell inside a cut, it now points at
the first frame after it: check that frame still shows the look to ignore, or pick a new `at`.

- Each line is spoken by macOS `say` (`--voice`, default Samantha, ~3 words per second;
  `--voice=none` for captions only). When a line still runs at the next line's `at`, the video
  holds the frame at that `at` until it ends, so speech never runs over a later step.
- A hold only waits at the next line's `at`: whatever happens between the previous line's `at`
  and that moment plays while the previous line is still speaking. A line anchored after its
  change therefore starts once the change is already over. `--plan` with `--regions` flags each
  line whose `at` falls just after a change started and prints the `at` to use instead.
- `--plan` also flags each hold longer than 1 s: the line before it is too long for its stretch.
  Shorten that line rather than accept the freeze.
- Read the narrated frames at each hold before delivering.
- Each line is burned in as a caption in a band of its own below the video, from its start
  until the next line starts, so captions never cover the recording: a 1280x720 cut comes out
  1280x816. `--caption-at=top` puts the band above, `--caption-height` sets its height (default
  96, two lines of text; keep it even for H.264), `--no-band` leaves the video its own size.
- `--vtt=<file>` writes the lines as WebVTT on the output's timeline; `--cues=<file>` writes
  them as JSON for a text track added from inline JS (see Deliver in `SKILL.md`).
