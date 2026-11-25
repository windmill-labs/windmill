// Shrinks the idle stretches of a silent screen recording (a terminal session, a browser take)
// to about a second each and marks every cut with a "N s later" note. It measures what changes,
// it does not guess: a stretch is idle when no watched region changes.
//
// usage: node cut-waits.mjs <in.mp4> <out.mp4> <regions.json> [options]
//   --plan          print the cuts and write the check sheets, render nothing
//   --min-gap=3     cut only stretches at least this long (s)
//   --head=0.7      keep this much of a stretch after the last change (s)
//   --tail=0.3      and this much before the next change (s)
//   --end-hold=5    keep this much after the last change, then end the video (s)
//   --label-y=12    top of the "N s later" note, in pixels
//   --cut=12.5-38   also remove this stretch, though the screen changes in it (repeatable)
//
// --cut is for a wait the measurement cannot see: an AI agent working, whose tool calls keep
// scrolling in while the viewer only waits for its answer. Take its bounds from activity.mjs,
// which prints when each region changes in seconds of <in.mp4>, never from wall-clock logs.
//
// regions.json lists the areas to watch, in pixels of <in.mp4>. A pixel changes when its gray
// level moves by more than 40 between two samples (10 per second); a region is active when more
// than `min` pixels change. Set `min` above what a blinking cursor changes (one cell, 170-210 px at
// font size 15) and below what typing changes (several cells per sample).
// `unlessLike` ignores a region's changes while a part of it still looks as it did at `at`
// seconds, e.g. a status line whose timer ticks while the app works but whose row also receives
// real output once the work is done.
//   [{ "name": "status", "x": 0, "y": 670, "w": 820, "h": 35, "min": 30,
//      "unlessLike": { "x": 40, "y": 675, "w": 72, "h": 22, "at": 40 } }]
//
// Writes <out>.regions.json, the regions with each unlessLike.at moved onto the output's timeline,
// for activity.mjs and narrate.mjs on the cut file.
// Writes <out>.cuts-<n>.png: for every cut, the last frame kept before it beside the first
// frame kept after it. Read them before trusting the cut: the two frames of a row must show the
// same content, apart from what ticks.
import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { die, launchBrowser, need, probe, sampleRegions } from './lib.mjs'

const [input, output, regionsFile, ...rest] = process.argv.slice(2)
if (!regionsFile) die('usage: node cut-waits.mjs <in.mp4> <out.mp4> <regions.json> [--plan] [--min-gap=3] ...')
need('ffmpeg', 'ffprobe')
const forced = rest
	.filter((a) => a.startsWith('--cut='))
	.map((a) => {
		const [from, to] = a.slice('--cut='.length).split('-').map(Number)
		if (!(to > from)) die(`bad ${a}: expected --cut=<from>-<to> in seconds`)
		return { from, to }
	})
const opt = Object.fromEntries(
	rest
		.filter((a) => !a.startsWith('--cut='))
		.map((a) => {
			const [k, v] = a.replace(/^--/, '').split('=')
			return [k, v === undefined ? true : Number(v)]
		})
)
const minGap = opt['min-gap'] ?? 3
const head = opt.head ?? 0.7
const tail = opt.tail ?? 0.3
const endHold = opt['end-hold'] ?? 5
const labelY = opt['label-y'] ?? 12
if (minGap < head + tail) die(`--min-gap must be at least --head + --tail (${head + tail} s)`)
const regions = JSON.parse(readFileSync(regionsFile, 'utf8'))
const { fps, duration, hasAudio } = probe(input)

// 1. Measure: the samples where any watched region changes.
const active = (await sampleRegions(input, regions)).filter((s) => s.active).map((s) => s.t)
if (!active.length) {
	console.error('nothing changes in the watched regions')
	process.exit(1)
}

// 2. Plan: shrink every stretch of at least minGap between two changes, and the ending.
const measured = []
if (active[0] >= minGap) measured.push({ from: head, to: active[0] - tail })
for (let i = 1; i < active.length; i++) {
	const [a, b] = [active[i - 1], active[i]]
	if (b - a >= minGap) measured.push({ from: a + head, to: b - tail })
}
// Cuts less than half a second apart become one: the sliver between them would only flash.
const cuts = []
for (const c of [...measured, ...forced].sort((a, b) => a.from - b.from)) {
	const prev = cuts.at(-1)
	if (prev && c.from <= prev.to + 0.5) prev.to = Math.max(prev.to, c.to)
	else cuts.push({ ...c })
}
const last = active.at(-1)
if (duration - last > endHold + 0.2) {
	const from = last + endHold
	const prev = cuts.at(-1)
	if (prev && prev.to >= from - 0.5) Object.assign(prev, { to: duration, end: true })
	else cuts.push({ from, to: duration, end: true })
}
// Cuts fall on frame times and the render keeps each segment half-open (select, below), so the
// output has exactly the planned frames and the times printed and mapped here are the rendered ones.
const snap = (t) => Math.round(t * fps) / fps
for (const c of cuts) Object.assign(c, { from: snap(c.from), to: c.end ? duration : snap(c.to) })
const kept = []
let cursor = 0
for (const c of cuts) {
	kept.push([cursor, c.from])
	cursor = c.to
}
if (cursor < duration) kept.push([cursor, duration])
let outAt = 0
cuts.forEach((c, k) => {
	outAt += kept[k][1] - kept[k][0]
	c.outAt = outAt
	c.label = `${Math.round(c.to - c.from)} s later`
})
const outDuration = kept.reduce((s, [a, b]) => s + b - a, 0)
const clock = (s) => `${Math.floor(s / 60)}:${(s % 60).toFixed(1).padStart(4, '0')}`
for (const c of cuts)
	console.log(
		`${c.end ? 'end' : 'cut'}  ${c.from.toFixed(1).padStart(6)} -> ${c.to.toFixed(1).padStart(6)}  ` +
			`${(c.to - c.from).toFixed(1).padStart(5)} s removed, at ${clock(c.outAt)} in the output` +
			(c.end ? '' : `  "${c.label}"`)
	)
console.log(`duration ${duration.toFixed(1)} s -> ${outDuration.toFixed(1)} s`)

// The regions for the cut file: an `unlessLike.at` is a time of the file it samples, so it moves
// with the cut. A time inside a cut moves to the cut's position in the output.
const toOutput = (t) => {
	let base = 0
	for (const [a, b] of kept) {
		if (t < a) return base
		if (t <= b) return base + t - a
		base += b - a
	}
	return outDuration
}
const outRegions = output.replace(/\.mp4$/, '') + '.regions.json'
writeFileSync(
	outRegions,
	JSON.stringify(
		regions.map((r) => (r.unlessLike ? { ...r, unlessLike: { ...r.unlessLike, at: +toOutput(r.unlessLike.at).toFixed(2) } } : r)),
		null,
		1
	)
)
console.log(`regions of the output: ${outRegions}`)

// 3. Check sheets: the frames on both sides of every cut.
const work = mkdtempSync(join(tmpdir(), 'cut-waits-'))
const shot = (t, file) =>
	execFileSync('ffmpeg', ['-v', 'error', '-y', '-ss', t.toFixed(3), '-i', input, '-frames:v', '1', '-vf', 'scale=640:-2', file])
const rows = cuts.filter((c) => !c.end)
for (let s = 0; s * 4 < rows.length; s++) {
	const files = []
	rows.slice(s * 4, s * 4 + 4).forEach((c, k) => {
		const a = join(work, `s${s}r${k}a.png`)
		const b = join(work, `s${s}r${k}b.png`)
		shot(Math.max(0, c.from - 1 / fps), a)
		shot(c.to, b)
		files.push(a, b)
	})
	const n = files.length / 2
	const graph =
		files.map((_, i) => (i % 2 ? '' : `[${i}][${i + 1}]hstack[r${i / 2}];`)).join('') +
		(n > 1 ? `${[...Array(n).keys()].map((i) => `[r${i}]`).join('')}vstack=inputs=${n}` : '[r0]null')
	const sheet = output.replace(/\.mp4$/, '') + `.cuts-${s + 1}.png`
	execFileSync('ffmpeg', ['-v', 'error', '-y', ...files.flatMap((f) => ['-i', f]), '-filter_complex', graph, sheet])
	console.log(`check: ${sheet}`)
}
if (opt.plan) process.exit(0)

// 4. Notes, drawn by the browser: Homebrew's ffmpeg has no text filters.
const browser = await launchBrowser()
const page = await browser.newPage({ viewport: { width: 600, height: 120 } })
const notes = new Map()
for (const c of rows) {
	if (notes.has(c.label)) continue
	await page.setContent(
		`<body style="margin:0;background:transparent"><div style="display:inline-block;padding:8px 16px;` +
			`border-radius:999px;background:rgba(17,17,17,.85);border:1px solid rgba(255,255,255,.3);` +
			`color:#fff;font:600 18px/1 system-ui,sans-serif"></div></body>`
	)
	await page.locator('div').evaluate((el, t) => (el.textContent = t), `⏩ ${c.label}`)
	const file = join(work, `note${notes.size}.png`)
	await page.locator('div').screenshot({ path: file, omitBackground: true })
	notes.set(c.label, file)
}
await browser.close()

// 5. Render: keep the segments, retime them, show each note for a moment after its cut.
const noteFiles = [...notes.values()]
// Half a frame below each bound, so a frame exactly on a bound is kept once, by the later segment.
const half = 0.5 / fps
const keep = kept.map(([a, b]) => `gte(t,${(a - half).toFixed(4)})*lt(t,${(b - half).toFixed(4)})`).join('+')
// The second fps= restores the frame rate FFmpeg 7's setpts clears; without it the overlays
// below fall back to 25 fps.
const filters = [`[0:v]fps=${fps},select='${keep}',setpts=N/(${fps}*TB),fps=${fps}[v0]`]
rows.forEach((c, k) => {
	const next = rows[k + 1]?.outAt ?? outDuration
	const e = Math.min(c.outAt + 1.4, next)
	const input = 1 + noteFiles.indexOf(notes.get(c.label))
	filters.push(
		`[v${k}][${input}:v]overlay=x=(W-w)/2:y=${labelY}:enable='between(t,${c.outAt.toFixed(3)},${e.toFixed(3)})'[v${k + 1}]`
	)
})
execFileSync(
	'ffmpeg',
	[
		'-v', 'error', '-y', '-i', input,
		...noteFiles.flatMap((f) => ['-i', f]),
		'-filter_complex', filters.join(';'),
		'-map', `[v${rows.length}]`, '-an',
		'-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-pix_fmt', 'yuv420p', '-movflags', '+faststart',
		output
	],
	{ stdio: ['ignore', 'inherit', 'inherit'] }
)
if (hasAudio) console.log('note: the input had audio; the output has none')
console.log(output)
