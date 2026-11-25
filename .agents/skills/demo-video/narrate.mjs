// Lays a spoken voice-over on a silent video: one text-to-speech line (macOS `say`) per entry,
// started at the moment of the video it describes. When a line still runs at the next line's
// moment, the video holds the frame of that moment until the line ends, so picture and speech
// stay together.
//
// usage: node narrate.mjs <in.mp4> <lines.json> <out.mp4> [options]
//   --plan            speak the lines and print where they land, render nothing
//   --regions=<file>  regions.json of <in.mp4>: warn about a line whose `at` falls just after
//                     a change started, since its caption would then land after that change
//   --voice=Samantha  macOS voice (say -v '?'); --voice=none speaks nothing and gives each
//                     line its reading time, for a machine without `say`
//   --rate=0          words per minute for say; 0 keeps the voice's own
//   --gap=0.4         silence between two lines (s)
//   --end-hold=1.5    keep the last frame this long after the last line (s)
//   --vtt=<file>      also write the lines as WebVTT captions, on the output's timeline
//   --cues=<file>     also write them as JSON [[start, end, text], ...], for a text track
//                     added from inline JS on a page that cannot serve .vtt files
//   --caption-height=96  height of the caption band added below the video (two lines of text)
//   --caption-at=top  put the band above the video instead
//   --no-band         burn in no captions: the output keeps the input's size
//
// Each line shows in the band from its start until the next line starts, so captions never
// cover the recording.
//
// lines.json: [{ "at": 0.4, "text": "One sentence about what is on screen." }, ...], `at` in
// seconds of <in.mp4>, increasing. Anchor a line a little before the change it describes: a
// hold freezes the frame at `at`, so a line anchored after its change starts only once that
// change has already played.
import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { captionImages, die, launchBrowser, need, probe, sampleRegions, speak } from './lib.mjs'

const [input, linesFile, output, ...rest] = process.argv.slice(2)
if (!output) die('usage: node narrate.mjs <in.mp4> <lines.json> <out.mp4> [--plan] [--voice=Samantha] [--gap=0.4] ...')
const opt = Object.fromEntries(
	rest.map((a) => {
		const [k, ...v] = a.replace(/^--/, '').split('=')
		return [k, v.length ? v.join('=') : true]
	})
)
const voice = opt.voice || 'Samantha'
need('ffmpeg', 'ffprobe', ...(voice === 'none' ? [] : ['say']))
const rate = Number(opt.rate || 0)
const gap = Number(opt.gap ?? 0.4)
const endHold = Number(opt['end-hold'] ?? 1.5)
const lines = JSON.parse(readFileSync(linesFile, 'utf8'))
lines.forEach((l, i) => {
	if (i && l.at < lines[i - 1].at) die(`line ${i + 1} starts before line ${i}: sort them by "at"`)
})

const vs = probe(input)
const { fps, duration } = vs

// 1. Speak every line.
const work = mkdtempSync(join(tmpdir(), 'narrate-'))
const cues = lines.map((l, i) => ({ ...l, ...speak(l.text, join(work, `line${i}.aiff`), { voice, rate }) }))

// 2. Place them. A line that would start before the previous one ends holds the frame at its
// own moment for the difference, which delays everything after it.
const holds = new Map()
const hold = (at, d) => {
	const near = [...holds.keys()].find((k) => Math.abs(k - at) < 1 / fps)
	holds.set(near ?? at, (holds.get(near ?? at) ?? 0) + d)
}
let shift = 0
let prevEnd = -Infinity
for (const c of cues) {
	c.start = c.at + shift
	c.held = 0
	// Under a frame of overlap only shortens the gap: a hold that short would show nothing.
	if (c.start < prevEnd + gap - 1 / fps) {
		c.held = prevEnd + gap - c.start
		hold(c.at, c.held)
		shift += c.held
		c.start += c.held
	}
	c.end = c.start + c.length
	prevEnd = c.end
}
let outDuration = duration + shift
if (prevEnd + endHold > outDuration) {
	hold(duration, prevEnd + endHold - outDuration)
	outDuration = prevEnd + endHold
}

// A change that starts in the 0.6 s before a line's `at` is one the line most likely describes.
const late = new Map()
if (opt.regions) {
	const regions = JSON.parse(readFileSync(opt.regions, 'utf8'))
	const samples = await sampleRegions(input, regions)
	const starts = []
	samples.forEach((s, i) => {
		if (!s.active) return
		const quietBefore = !samples.slice(Math.max(0, i - 5), i).some((p) => p.active)
		if (quietBefore)
			starts.push({ t: s.t, region: regions.find((r, k) => !s.suppressed[k] && s.changed[k] > r.min).name })
	})
	cues.forEach((c, i) => {
		const s = starts.find((s) => s.t > c.at - 0.6 && s.t <= c.at)
		if (s) late.set(i, s)
	})
}

const clock = (s) => `${Math.floor(s / 60)}:${(s % 60).toFixed(1).padStart(4, '0')}`
for (const [i, c] of cues.entries()) {
	console.log(
		`${clock(c.start)}-${clock(c.end)}  at ${c.at.toFixed(1).padStart(5)}` +
			(c.held ? `  holds ${c.held.toFixed(1)} s` : '') +
			`  ${c.text}`
	)
	if (c.held > 1)
		console.log(`    ^ line ${i} runs ${c.held.toFixed(1)} s into this one's moment: shorten line ${i}`)
	const s = late.get(i)
	if (s)
		console.log(`    ^ ${s.region} starts changing at ${s.t.toFixed(1)}, before this line: anchor it at ${Math.max(0, s.t - 0.3).toFixed(1)}`)
}
console.log(`duration ${duration.toFixed(1)} s -> ${outDuration.toFixed(1)} s`)

// The band shows each caption until the next line starts.
const shown = cues.map((c, k) => [c.start, k + 1 < cues.length ? cues[k + 1].start : outDuration, c.text])
if (opt.vtt) {
	const stamp = (s) => {
		const ms = Math.round(s * 1000)
		const p = (n, w = 2) => String(n).padStart(w, '0')
		return `${p(Math.floor(ms / 3600000))}:${p(Math.floor(ms / 60000) % 60)}:${p(Math.floor(ms / 1000) % 60)}.${p(ms % 1000, 3)}`
	}
	writeFileSync(
		opt.vtt,
		'WEBVTT\n\n' + cues.map((c, k) => `${k + 1}\n${stamp(c.start)} --> ${stamp(c.end)}\n${c.text}\n`).join('\n')
	)
	console.log(`captions: ${opt.vtt}`)
}
if (opt.cues) {
	writeFileSync(opt.cues, JSON.stringify(shown.map(([s, e, t]) => [+s.toFixed(2), +e.toFixed(2), t])))
	console.log(`cues: ${opt.cues}`)
}
if (opt.plan) process.exit(0)

// 3. Caption band images.
const band = opt['no-band'] ? 0 : Number(opt['caption-height'] ?? 96)
if (band % 2 || vs.height % 2 || vs.width % 2) die('video width, height and --caption-height must all be even for H.264')
const top = opt['caption-at'] === 'top'
if (band) {
	const browser = await launchBrowser()
	try {
		const images = await captionImages(
			browser,
			cues.map((c) => c.text),
			{ width: vs.width, height: band, dir: work }
		)
		images.forEach((f, k) => (cues[k].image = f))
	} finally {
		await browser.close()
	}
}

// 4. Render: the video cut at every hold, each piece padded with its last frame, then the
// lines delayed to their start and mixed.
const filters = []
const pieces = []
let from = 0
for (const p of [...holds.keys()].sort((a, b) => a - b)) {
	// fps= restores the frame rate FFmpeg 7's setpts clears; without it tpad adds no frames.
	filters.push(
		`[0:v]trim=start=${from.toFixed(3)}:end=${p.toFixed(3)},setpts=PTS-STARTPTS,fps=${fps},` +
			`tpad=stop_mode=clone:stop_duration=${holds.get(p).toFixed(3)}[p${pieces.length}]`
	)
	pieces.push(`[p${pieces.length}]`)
	from = p
}
if (duration - from > 1 / fps) {
	filters.push(`[0:v]trim=start=${from.toFixed(3)},setpts=PTS-STARTPTS[p${pieces.length}]`)
	pieces.push(`[p${pieces.length}]`)
}
filters.push(
	`${pieces.join('')}concat=n=${pieces.length}:v=1:a=0,fps=${fps}` +
		(band ? `,pad=${vs.width}:${vs.height + band}:0:${top ? band : 0}:color=#111111[c0]` : `,format=yuv420p[v]`)
)
const spoken = cues.filter((c) => c.file)
if (band) {
	const firstImage = spoken.length + 1
	shown.forEach(([s, e], k) => {
		filters.push(
			`[c${k}][${firstImage + k}:v]overlay=0:${top ? 0 : vs.height}:` +
				`enable='between(t,${s.toFixed(3)},${e.toFixed(3)})'[c${k + 1}]`
		)
	})
	filters.push(`[c${cues.length}]format=yuv420p[v]`)
}
spoken.forEach((c, k) => filters.push(`[${k + 1}:a]adelay=${Math.round(c.start * 1000)}:all=1[a${k}]`))
if (spoken.length)
	filters.push(`${spoken.map((_, k) => `[a${k}]`).join('')}amix=inputs=${spoken.length}:normalize=0,apad[a]`)
execFileSync(
	'ffmpeg',
	[
		'-v', 'error', '-y', '-i', input,
		...spoken.flatMap((c) => ['-i', c.file]),
		...(band ? cues.flatMap((c) => ['-i', c.image]) : []),
		'-filter_complex', filters.join(';'),
		'-map', '[v]', ...(spoken.length ? ['-map', '[a]', '-c:a', 'aac', '-b:a', '128k'] : []),
		'-t', outDuration.toFixed(3),
		'-c:v', 'libx264', '-crf', '20', '-preset', 'medium', '-movflags', '+faststart',
		output
	],
	{ stdio: ['ignore', 'inherit', 'inherit'] }
)
console.log(output)
