// Derives the fork-chip accent palette from a workspace color: the default
// blue's light/dark bg+text profile, re-hued. Lightness is fixed per role so
// any picked hue stays readable; saturation follows the user color within a
// safe band so grayish picks yield grayish chips.
//
// The badge filled with the hue is the exception, because white is written on
// it and a fixed lightness cannot carry white across the wheel: HSL lightness
// is not perceived brightness, so a yellow and a purple at the same L differ by
// several stops of luminance. Its lightness is solved instead — the lightest
// shade of the hue that still holds white at WCAG AA.

/** WCAG AA for small text: the badge's own label is what is being read. */
const TEXT_MIN_CONTRAST = 4.5
/** White, which the badge's label is written in. */
const WHITE_LUMINANCE = 1

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

/** The same string with lightness floored: a solved lightness is the lightest value that clears
 *  its contrast target, so rounding the whole percent up would spend the margin just bought. */
function hslFloorL(h: number, s: number, l: number): string {
	return `hsl(${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.floor(l * 100)}%)`
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
	const c = (1 - Math.abs(2 * l - 1)) * s
	const hp = (((h % 360) + 360) % 360) / 60
	const x = c * (1 - Math.abs((hp % 2) - 1))
	const [r, g, b] =
		hp < 1
			? [c, x, 0]
			: hp < 2
				? [x, c, 0]
				: hp < 3
					? [0, c, x]
					: hp < 4
						? [0, x, c]
						: hp < 5
							? [x, 0, c]
							: [c, 0, x]
	const m = l - c / 2
	return [r + m, g + m, b + m]
}

function relativeLuminance(r: number, g: number, b: number): number {
	const f = (x: number) => (x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4)
	return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

function contrast(a: number, b: number): number {
	return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

/** Luminance of what the browser paints, which is neither the exact numbers nor the `hsl()` string
 *  alone: the string carries whole percents, and the channels it resolves to are 8-bit. Solving
 *  against the unrounded channels leaves the chosen lightness up to 0.025 short of the target once
 *  painted, which is enough to miss AA on a hue like #993388. */
function paintedLuminance(h: number, s: number, l: number): number {
	const [r, g, b] = hslToRgb(Math.round(h), Math.round(s * 100) / 100, Math.floor(l * 100) / 100)
	const q = (x: number) => Math.round(x * 255) / 255
	return relativeLuminance(q(r), q(g), q(b))
}

/**
 * The lightest L in [0, hi] whose color contrasts at least `target` against `against`. Luminance
 * rises with L at a fixed hue and saturation, so the predicate flips once and a bisection finds
 * the edge; 12 steps put it within 0.03 of a percent, finer than the whole percent the string
 * carries.
 */
function lightestAtContrast(
	h: number,
	s: number,
	hi: number,
	against: number,
	target: number
): number {
	let lo = 0
	let best = 0
	for (let i = 0; i < 12; i++) {
		const mid = (lo + hi) / 2
		if (contrast(paintedLuminance(h, s, mid), against) >= target) {
			best = mid
			lo = mid
		} else {
			hi = mid
		}
	}
	return best
}

/**
 * Inline-style string setting the `--fork-accent-*` custom properties a colored
 * fork chip consumes (see WorkspaceScopeTrigger). Returns undefined for an
 * unparsable color, letting the chip fall back to the default accent.
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
	// The filled badge inside the chip: the hue at near-full strength, as light as it can be while
	// white still reads on it. Same in both themes — it is the one thing on the line that is a
	// block of color rather than a tint, so it needs no second version.
	const badgeSat = clamp(s, 0.35, 0.9)
	const badge = hslFloorL(
		h,
		badgeSat,
		lightestAtContrast(h, badgeSat, 0.7, WHITE_LUMINANCE, TEXT_MIN_CONTRAST)
	)
	return `--fork-accent-bg: ${lightBg}; --fork-accent-text: ${lightText}; --fork-accent-badge: ${badge}; --fork-accent-bg-dark: ${darkBg}; --fork-accent-text-dark: ${darkText};`
}
