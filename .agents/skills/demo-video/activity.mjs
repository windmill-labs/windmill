// Prints when each watched region of a recording changes, in seconds of the file itself, so
// `--cut=` ranges and narration `at` times come from the picture rather than from wall-clock
// logs (a vhs mp4 runs shorter than the session it recorded).
//
// usage: node activity.mjs <in.mp4> <regions.json> [from] [to] [--raw]
//
// Same sampling and regions.json as cut-waits.mjs (10 samples per second, `min`, `unlessLike`).
// Each region gets one line of runs of consecutive changes, `12.3-14.1(952)`: first and last
// changing sample, and the most pixels changed in one sample. --raw prints every sample in which
// anything changed, marked * when it counts (above `min`) and ~ when `unlessLike` suppressed it.
import { readFileSync } from 'node:fs'
import { die, need, sampleRegions, SAMPLES } from './lib.mjs'

const args = process.argv.slice(2)
const raw = args.includes('--raw')
const [input, regionsFile, from, to] = args.filter((a) => a !== '--raw')
if (!regionsFile) die('usage: node activity.mjs <in.mp4> <regions.json> [from] [to] [--raw]')
need('ffmpeg', 'ffprobe')
const regions = JSON.parse(readFileSync(regionsFile, 'utf8'))
const samples = await sampleRegions(input, regions, {
	from: from === undefined ? 0 : Number(from),
	to: to === undefined ? undefined : Number(to)
})

for (const [k, r] of regions.entries()) {
	const hits = samples.filter((s) => !s.suppressed[k] && s.changed[k] > r.min)
	if (raw) {
		const all = samples.filter((s) => s.changed[k] > 0)
		const mark = (s) => (s.suppressed[k] ? '~' : s.changed[k] > r.min ? '*' : '')
		console.log(`${r.name}: ${all.map((s) => `${s.t.toFixed(1)}(${s.changed[k]})${mark(s)}`).join(' ')}`)
		continue
	}
	const runs = []
	for (const s of hits) {
		const run = runs.at(-1)
		if (run && s.t - run.to <= 1.5 / SAMPLES) {
			run.to = s.t
			run.max = Math.max(run.max, s.changed[k])
		} else runs.push({ from: s.t, to: s.t, max: s.changed[k] })
	}
	const fmt = (run) =>
		(run.to > run.from ? `${run.from.toFixed(1)}-${run.to.toFixed(1)}` : run.from.toFixed(1)) + `(${run.max})`
	console.log(`${r.name}: ${runs.map(fmt).join(' ') || '-'}`)
}
