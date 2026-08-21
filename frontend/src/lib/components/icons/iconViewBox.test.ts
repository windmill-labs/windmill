import { describe, expect, it } from 'vitest'
import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Brand marks share the box lucide draws in: a square viewBox centred on the artwork, sized so
// the painted artwork spans 22/24 of it. Squareness is the half that can be checked without a
// renderer — the 22/24 ratio needs rasterising, so it is enforced by review, not here. A brand's
// own viewBox pasted from a press kit is the way this drifts, and it is almost always non-square.
const iconsDir = dirname(fileURLToPath(import.meta.url))

function svelteFiles(dir: string): string[] {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = join(dir, entry.name)
		if (entry.isDirectory()) return svelteFiles(full)
		return entry.name.endsWith('.svelte') ? [full] : []
	})
}

describe('icon viewBox', () => {
	const boxes = svelteFiles(iconsDir).flatMap((file) => {
		const source = readFileSync(file, 'utf8')
		return [...source.matchAll(/viewBox="([^"]*)"/g)].map((m) => ({
			icon: file.slice(iconsDir.length + 1),
			viewBox: m[1]
		}))
	})

	it('is square on every icon', () => {
		const offenders = boxes.filter(({ viewBox }) => {
			const parts = viewBox
				.trim()
				.split(/[\s,]+/)
				.map(Number)
			if (parts.length !== 4 || parts.some((n) => !Number.isFinite(n))) return true
			const [, , width, height] = parts
			return !(width > 0) || width !== height
		})
		expect(offenders.map((o) => `${o.icon}: ${o.viewBox}`)).toEqual([])
	})

	// Guards the walk itself: a broken path would make the assertion above pass over nothing.
	it('finds the icon library', () => {
		expect(boxes.length).toBeGreaterThan(300)
	})
})
