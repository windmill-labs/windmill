#!/usr/bin/env node
// Usage: node record.mjs <scenario.mjs> [out.mp4]
// Prints the mp4 path on success; on a failed step, the step and a screenshot path.
import { execFileSync } from 'node:child_process'
import { mkdtempSync, readdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve, dirname, basename } from 'node:path'
import { pathToFileURL } from 'node:url'
import { captionImages, die, launchBrowser, need, speak } from './lib.mjs'

const [scenarioPath, outArg] = process.argv.slice(2)
if (!scenarioPath) die('usage: node record.mjs <scenario.mjs> [out.mp4]')
const sc = (await import(pathToFileURL(resolve(scenarioPath)).href)).default
const out = resolve(outArg ?? join(dirname(resolve(scenarioPath)), basename(scenarioPath, '.mjs') + '.mp4'))
const work = mkdtempSync(join(tmpdir(), 'demo-video-'))
const viewport = sc.viewport ?? { width: 1280, height: 720 }
const voice = sc.voice === false ? 'none' : (sc.voice ?? 'Samantha')
const BAND = sc.captionHeight ?? 96
if (viewport.width % 2 || (viewport.height + BAND) % 2) die('viewport width and height + captionHeight must be even for H.264')
need('ffmpeg', 'ffprobe', ...(voice === 'none' ? [] : ['say']))

// 1. Narration clips, before the browser starts, so recording time is not spent on TTS.
const clips = sc.steps.map((s, i) => {
	if (!s.say) return null
	const { file, length } = speak(s.say, join(work, `say${i}.aiff`), { voice })
	return { file, ms: Math.round(length * 1000) }
})

const browser = await launchBrowser()
try {
	// 2. Setup (login, seeding) runs unrecorded; its cookies carry into the recorded context.
	let storageState
	if (sc.setup) {
		const ctx = await browser.newContext({ viewport, baseURL: sc.baseURL })
		try {
			await sc.setup({ context: ctx, page: await ctx.newPage(), baseURL: sc.baseURL })
		} catch (err) {
			die(`setup failed: ${err.message.split('\n')[0]}`)
		}
		storageState = await ctx.storageState()
		await ctx.close()
	}
	const ctx = await browser.newContext({
		viewport,
		storageState,
		baseURL: sc.baseURL,
		recordVideo: { dir: work, size: viewport }
	})
	const page = await ctx.newPage()
	const t0rec = Date.now()

	// Helpers handed to each step: a visible ring on what is clicked, so the viewer can follow.
	const ring = (loc) =>
		loc
			.evaluate((el) => {
				const r = el.getBoundingClientRect()
				const d = document.createElement('div')
				d.style.cssText = `position:fixed;left:${r.left - 4}px;top:${r.top - 4}px;width:${r.width + 8}px;height:${r.height + 8}px;border:3px solid #f43f5e;border-radius:8px;z-index:2147483646;pointer-events:none`
				document.body.appendChild(d)
				setTimeout(() => d.remove(), 900)
			})
			.catch(() => {})
	const loc = (t) => (typeof t === 'string' ? page.locator(t).first() : t)
	const helpers = {
		page,
		click: async (t) => {
			const l = loc(t)
			await l.scrollIntoViewIfNeeded()
			await ring(l)
			await page.waitForTimeout(500)
			await l.click()
		},
		hover: async (t) => {
			const l = loc(t)
			await l.scrollIntoViewIfNeeded()
			await ring(l)
			await l.hover()
		},
		wait: (ms) => page.waitForTimeout(ms)
	}

	if (sc.start) await page.goto(sc.start, { waitUntil: 'networkidle' })
	await page.waitForTimeout(sc.settle ?? 800)
	const trimStart = Date.now() - t0rec
	const cues = []
	// Each caption shows from its step's start to the next captioned step's start.
	const captions = []

	// 3. A step's narration starts before its action, and the step stays on screen until both
	//    are done, so a caption never lands after what it describes.
	for (const [i, s] of sc.steps.entries()) {
		const at = Date.now() - t0rec
		if (s.say) {
			if (captions.length) captions.at(-1).end = at - trimStart
			captions.push({ text: s.say, start: at - trimStart })
		}
		try {
			if (s.do) await s.do(helpers)
		} catch (err) {
			const shot = join(work, `failed-step${i}.png`)
			await page.screenshot({ path: shot }).catch(() => {})
			console.error(`step ${i} failed (${s.say ?? 'no narration'}): ${err.message.split('\n')[0]}`)
			console.error(`screenshot: ${shot}`)
			process.exitCode = 1
			break
		}
		const left = (clips[i]?.ms ?? 0) + (sc.pad ?? 400) - (Date.now() - t0rec - at)
		if (left > 0) await page.waitForTimeout(left)
		if (s.hold) await page.waitForTimeout(s.hold)
		if (clips[i]?.file) cues.push({ file: clips[i].file, delay: at - trimStart })
	}
	await page.waitForTimeout(600)
	const total = (Date.now() - t0rec - trimStart) / 1000
	if (captions.length) captions.at(-1).end = total * 1000
	await ctx.close()
	if (process.exitCode) process.exit()

	// 4. Captions in a band of their own beside the page, so they never cover it.
	const images = await captionImages(
		browser,
		captions.map((c) => c.text),
		{ width: viewport.width, height: BAND, dir: work }
	)

	// 5. Mux: cut the pre-roll, add the caption band, show each caption during its step, lay
	//    each narration clip at its step's start, encode an mp4.
	const video = join(work, readdirSync(work).find((f) => f.endsWith('.webm')))
	const args = ['-y', '-loglevel', 'error', '-ss', String(trimStart / 1000), '-i', video]
	images.forEach((f) => args.push('-i', f))
	cues.forEach((c) => args.push('-i', c.file))
	const top = sc.captionAt === 'top'
	const filters = [
		`[0:v]pad=${viewport.width}:${viewport.height + BAND}:0:${top ? BAND : 0}:color=#111111[v0]`
	]
	captions.forEach((c, k) => {
		const s = (c.start / 1000).toFixed(2)
		const e = (c.end / 1000).toFixed(2)
		filters.push(
			`[v${k}][${k + 1}:v]overlay=0:${top ? 0 : viewport.height}:enable='between(t,${s},${e})'[v${k + 1}]`
		)
	})
	const audioBase = captions.length + 1
	cues.forEach((c, k) => filters.push(`[${audioBase + k}:a]adelay=${c.delay}:all=1[a${k}]`))
	if (cues.length) {
		filters.push(`${cues.map((_, k) => `[a${k}]`).join('')}amix=inputs=${cues.length}:normalize=0[a]`)
	}
	args.push('-filter_complex', filters.join(';'), '-map', `[v${captions.length}]`)
	if (cues.length) args.push('-map', '[a]', '-c:a', 'aac')
	args.push('-t', total.toFixed(2), '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', out)
	execFileSync('ffmpeg', args, { stdio: ['ignore', 'inherit', 'inherit'] })
	console.log(out)
} finally {
	await browser.close()
}
