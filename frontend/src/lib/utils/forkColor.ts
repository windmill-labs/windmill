// Derives the fork-chip accent palette from a workspace color.
//
// The default (blue) chip uses four theme tokens — light bg #ebefff / text
// #2652df, dark bg #33384e / text #c7cefc. A custom fork color re-creates that
// profile at the color's hue: pale tinted background + readable saturated text
// in light mode, desaturated dark background + pastel text in dark mode.
// Lightness is fixed per role (not taken from the user color) so any picked
// hue stays readable; saturation follows the user color within a safe band so
// grayish picks yield grayish chips.

function hexToHsl(hex: string): { h: number; s: number; l: number } | undefined {
	const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim())
	if (!m) return undefined
	let c = m[1]
	if (c.length === 3) c = [...c].map((x) => x + x).join('')
	const r = parseInt(c.slice(0, 2), 16) / 255
	const g = parseInt(c.slice(2, 4), 16) / 255
	const b = parseInt(c.slice(4, 6), 16) / 255
	const max = Math.max(r, g, b)
	const min = Math.min(r, g, b)
	const l = (max + min) / 2
	const d = max - min
	if (d === 0) return { h: 0, s: 0, l }
	const s = d / (1 - Math.abs(2 * l - 1))
	let h: number
	if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60
	else if (max === g) h = ((b - r) / d + 2) * 60
	else h = ((r - g) / d + 4) * 60
	return { h, s, l }
}

function clamp(x: number, lo: number, hi: number): number {
	return Math.min(hi, Math.max(lo, x))
}

function hsl(h: number, s: number, l: number): string {
	return `hsl(${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`
}

/**
 * Inline-style string setting the four `--fork-accent-*` custom properties a
 * colored fork chip consumes (see WorkspaceScopeTrigger). Returns undefined
 * for an unparsable color, letting the chip fall back to the default accent.
 */
export function forkAccentStyle(color: string | undefined): string | undefined {
	if (!color) return undefined
	const parsed = hexToHsl(color)
	if (!parsed) return undefined
	const { h, s } = parsed
	const lightBg = hsl(h, clamp(s, 0.3, 1), 0.955)
	const lightText = hsl(h, clamp(s, 0.3, 0.8), 0.42)
	const darkBg = hsl(h, clamp(s * 0.35, 0.08, 0.25), 0.26)
	const darkText = hsl(h, clamp(s, 0.3, 0.9), 0.86)
	return `--fork-accent-bg: ${lightBg}; --fork-accent-text: ${lightText}; --fork-accent-bg-dark: ${darkBg}; --fork-accent-text-dark: ${darkText};`
}
