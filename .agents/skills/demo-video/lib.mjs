// Shared by the demo-video scripts, and imported by the browser scripts written for a terminal
// take: `import { launchBrowser, waitForOutput } from '<repo>/.agents/skills/demo-video/lib.mjs'`.
import { createRequire } from 'node:module'
import { execFileSync, spawn } from 'node:child_process'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

export const CODEX_SESSIONS = join(homedir(), '.codex/sessions')
export const CLAUDE_PROJECTS = join(homedir(), '.claude/projects')

export function die(message) {
	console.error(message)
	process.exit(2)
}

const HINTS = {
	ffmpeg: 'brew install ffmpeg',
	ffprobe: 'brew install ffmpeg',
	vhs: 'brew install vhs',
	say: 'macOS only; use voice: false in a scenario or --voice=none for captions without speech'
}
export function need(...commands) {
	for (const c of commands) {
		try {
			execFileSync('which', [c], { stdio: 'ignore' })
		} catch {
			die(`${c} not found: ${HINTS[c] ?? 'install it'}`)
		}
	}
}

// The skill pins its own Playwright rather than using frontend/'s, whose older pin may not
// install its browser on a current Node.
const SKILL = dirname(fileURLToPath(import.meta.url))
export function playwright() {
	try {
		return createRequire(import.meta.url)('playwright')
	} catch {
		die(`Playwright not found: run \`npm ci --prefix ${SKILL}\``)
	}
}
export async function launchBrowser(options) {
	try {
		return await playwright().chromium.launch(options)
	} catch (err) {
		if (/Executable doesn't exist/.test(err.message))
			die(`Playwright's Chromium is not installed: run \`npx --prefix ${SKILL} playwright install chromium\``)
		throw err
	}
}

export function probe(file) {
	const info = JSON.parse(
		execFileSync('ffprobe', ['-v', 'error', '-show_streams', '-show_format', '-of', 'json', file])
	)
	const vs = info.streams.find((s) => s.codec_type === 'video')
	const [num, den] = (vs?.r_frame_rate ?? '0/1').split('/').map(Number)
	return {
		width: vs?.width,
		height: vs?.height,
		fps: num / den,
		duration: Number(info.format.duration),
		hasAudio: info.streams.some((s) => s.codec_type === 'audio')
	}
}

// One narration line as speech. With voice 'none' nothing is spoken and the line lasts as long
// as it takes to read.
export function speak(text, file, { voice, rate = 0 }) {
	if (voice === 'none') return { file: null, length: Math.max(1.5, text.split(/\s+/).length / 2.7) }
	execFileSync('say', ['-v', voice, ...(rate ? ['-r', String(rate)] : []), '-o', file, text])
	return { file, length: probe(file).duration }
}

// Caption band images, drawn by the browser: Homebrew's ffmpeg has no text filters.
export async function captionImages(browser, texts, { width, height, dir }) {
	const page = await browser.newPage({ viewport: { width, height } })
	const files = []
	for (const [k, text] of texts.entries()) {
		await page.setContent(
			`<body style="margin:0;height:${height}px;display:flex;align-items:center;justify-content:center;background:#111;">` +
				`<div style="max-width:88%;color:#fff;font:500 20px/1.35 system-ui,sans-serif;text-align:center;"></div></body>`
		)
		await page.locator('div').evaluate((el, t) => (el.textContent = t), text)
		files.push(join(dir, `caption${k}.png`))
		await page.screenshot({ path: files.at(-1) })
	}
	await page.close()
	return files
}

// Samples <input> 10 times a second inside [from, to] and returns, for each sample, how many
// pixels of each region changed since the previous one. A pixel changes when its gray level
// moves by more than 40. `suppressed[k]` is true while region k's `unlessLike` part still looks
// as it did at `unlessLike.at`; `active` is whether any region changed by more than its `min`.
export const SAMPLES = 10
export async function sampleRegions(input, regions, { from = 0, to } = {}) {
	const { width: W, height: H } = probe(input)
	const grayFrame = (t) =>
		execFileSync(
			'ffmpeg',
			['-v', 'error', '-ss', String(t), '-i', input, '-frames:v', '1', '-f', 'rawvideo', '-pix_fmt', 'gray', '-'],
			{ maxBuffer: W * H * 2 }
		)
	const lit = (frame, a) => {
		const bits = new Uint8Array(a.w * a.h)
		for (let y = 0; y < a.h; y++)
			for (let x = 0; x < a.w; x++) bits[y * a.w + x] = frame[(a.y + y) * W + a.x + x] > 70 ? 1 : 0
		return bits
	}
	const refs = regions.map((r) => {
		if (!r.unlessLike) return null
		const ref = lit(grayFrame(r.unlessLike.at), r.unlessLike)
		return { ref, lit: ref.reduce((s, b) => s + b, 0) }
	})
	const looksLike = (frame, k) => {
		const bits = lit(frame, regions[k].unlessLike)
		let diff = 0
		for (let i = 0; i < bits.length; i++) diff += bits[i] !== refs[k].ref[i]
		return diff < 0.25 * Math.max(refs[k].lit, 1)
	}
	const samples = []
	await new Promise((done, fail) => {
		const range = ['-ss', String(from), ...(to !== undefined ? ['-to', String(to)] : [])]
		const ff = spawn('ffmpeg', ['-v', 'error', ...range, '-i', input, '-vf', `fps=${SAMPLES},format=gray`, '-f', 'rawvideo', '-'])
		const size = W * H
		let buf = Buffer.alloc(0)
		let prev = null
		let prevLike = []
		let n = 0
		ff.stdout.on('data', (chunk) => {
			buf = Buffer.concat([buf, chunk])
			while (buf.length >= size) {
				const frame = Buffer.from(buf.subarray(0, size))
				buf = buf.subarray(size)
				const like = regions.map((r, k) => (r.unlessLike ? looksLike(frame, k) : false))
				if (prev) {
					const changed = regions.map((r) => {
						let c = 0
						for (let y = r.y; y < r.y + r.h; y++)
							for (let x = r.x, i = y * W + r.x; x < r.x + r.w; x++, i++) {
								const d = frame[i] - prev[i]
								if (d > 40 || d < -40) c++
							}
						return c
					})
					const suppressed = like.map((l, k) => l && prevLike[k])
					const active = regions.some((r, k) => !suppressed[k] && changed[k] > r.min)
					samples.push({ t: from + n / SAMPLES, changed, suppressed, active })
				}
				prev = frame
				prevLike = like
				n++
			}
		})
		ff.on('close', (code) => (code === 0 ? done() : fail(new Error(`ffmpeg exited ${code}`))))
	})
	return samples
}

// Waits until a file under `dirs` that was written after `since` contains `pattern`, and returns
// the match. Directories that do not exist yet are looked for again on every poll, so a log
// folder the agent creates during the take (Codex's per-day folder) is found once it appears.
// Logs are JSON: end the pattern before `"` and `\`. `&` is turned back into `&` first.
export async function waitForOutput(pattern, { dirs = [CODEX_SESSIONS], since = Date.now(), timeout = 300_000, every = 1000 } = {}) {
	const deadline = Date.now() + timeout
	const walk = (dir, out) => {
		if (!existsSync(dir)) return out
		for (const e of readdirSync(dir, { withFileTypes: true })) {
			const p = join(dir, e.name)
			if (e.isDirectory()) walk(p, out)
			else if (e.isFile()) out.push(p)
		}
		return out
	}
	while (Date.now() < deadline) {
		for (const dir of dirs)
			for (const file of walk(dir, [])) {
				try {
					if (statSync(file).mtimeMs < since) continue
					const m = readFileSync(file, 'utf8').replace(/\\u0026/g, '&').match(pattern)
					if (m) return m
				} catch {
					// written or removed while being read: look again on the next poll
				}
			}
		await new Promise((r) => setTimeout(r, every))
	}
	throw new Error(`no match for ${pattern} in ${dirs.join(', ')} after ${Math.round(timeout / 1000)} s`)
}
