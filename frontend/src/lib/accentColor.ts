import tokensFile from './assets/tokens/tokens.json'
import githubDarkTokens from './assets/tokens/githubDark.json'
import { darkModeName, lightModeName } from './assets/tokens/colorTokensConfig'

/** Instance-wide accent color, `#rrggbb` (mirrors `ACCENT_COLOR_SETTING` in the backend). */
export const ACCENT_COLOR_SETTING = 'accent_color'

const ACCENT_TOKENS = [
	'surface-accent-primary',
	'surface-accent-hover',
	'surface-accent-clicked',
	'surface-accent-selected',
	'surface-accent-secondary',
	'surface-accent-secondary-hover',
	'surface-accent-secondary-clicked',
	'text-accent',
	'border-accent',
	'border-selected'
] as const

const STYLE_ID = 'wm-accent-color'
const ACCENT_ATTR = 'data-wm-accent'

/**
 * The stored setting as a color, or `undefined` for "use the default theme".
 *
 * The backend already rejects anything but `#rrggbb`, but the value is interpolated into a
 * stylesheet, so this is the last place that can refuse a row written around the validator.
 */
export function parseAccentColor(value: unknown): string | undefined {
	return typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value)
		? value.toLowerCase()
		: undefined
}

type Oklch = [l: number, c: number, h: number]

function srgbToLinear(v: number): number {
	return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4)
}

function linearToSrgb(v: number): number {
	return v <= 0.0031308 ? v * 12.92 : 1.055 * Math.pow(v, 1 / 2.4) - 0.055
}

function hexToOklch(hex: string): Oklch {
	const [r, g, b] = [1, 3, 5].map((i) => srgbToLinear(parseInt(hex.slice(i, i + 2), 16) / 255))
	const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
	const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
	const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
	const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s
	const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
	const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
	return [L, Math.hypot(A, B), Math.atan2(B, A)]
}

/** Linear sRGB channels, possibly out of [0, 1] when the color is outside the gamut. */
function oklchToLinearRgb([L, C, h]: Oklch): [number, number, number] {
	const A = C * Math.cos(h)
	const B = C * Math.sin(h)
	const l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3
	const m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3
	const s = (L - 0.0894841775 * A - 1.291485548 * B) ** 3
	return [
		4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
		-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
		-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s
	]
}

/** `r g b` triplet (the format the `--color-*` variables use), chroma reduced into gamut. */
function oklchToRgbTriplet([L, C, h]: Oklch): string {
	const inGamut = (c: number) =>
		oklchToLinearRgb([L, c, h]).every((v) => v >= -1e-4 && v <= 1 + 1e-4)
	let lo = 0
	let hi = C
	if (!inGamut(C)) {
		for (let i = 0; i < 20; i++) {
			const mid = (lo + hi) / 2
			if (inGamut(mid)) lo = mid
			else hi = mid
		}
	} else lo = C
	return oklchToLinearRgb([L, lo, h])
		.map((v) => Math.round(Math.min(1, Math.max(0, linearToSrgb(v))) * 255))
		.join(' ')
}

// Below this chroma the chosen color reads as grey, and the accents fade with it.
const FULL_CHROMA = 0.1

/**
 * Move each accent token to the chosen hue while keeping its own lightness, so every
 * text/background pairing keeps the contrast the default palette was designed with.
 * Taking the chosen color verbatim would put white text on a pale yellow button.
 */
function recolor(tokens: Record<string, string>, accent: Oklch): string {
	const chromaScale = Math.min(1, accent[1] / FULL_CHROMA)
	return ACCENT_TOKENS.map((name) => {
		const [l, c] = hexToOklch(tokens[name])
		return `--color-${name}: ${oklchToRgbTriplet([l, c * chromaScale, accent[2]])};`
	}).join('')
}

/** The accent overrides for every theme, or `undefined` when `value` is not `#rrggbb`. */
export function accentStylesheet(value: unknown): string | undefined {
	const color = parseAccentColor(value)
	if (color == undefined) return undefined
	const accent = hexToOklch(color)
	// `:root[attr]` outranks the `html`, `html.dark` and `html.dark.github-dark` rules
	// that define the default tokens, whatever order the stylesheets load in.
	return [
		`:root[${ACCENT_ATTR}]{${recolor(tokensFile.tokens[lightModeName], accent)}` +
			`--sidebar-bg-light:color-mix(in oklab, ${color} 18%, var(--sidebar-bg-light-base));}`,
		`:root[${ACCENT_ATTR}].dark{${recolor(tokensFile.tokens[darkModeName], accent)}` +
			`--sidebar-bg-dark:color-mix(in oklab, ${color} 22%, var(--sidebar-bg-dark-base));}`,
		`:root[${ACCENT_ATTR}].dark.github-dark{${recolor(githubDarkTokens, accent)}}`
	].join('\n')
}

/** Apply the instance accent to the whole document, or restore the default with `undefined`. */
export function applyAccentColor(value: string | undefined): void {
	const root = document.documentElement
	let style = document.getElementById(STYLE_ID)
	const css = accentStylesheet(value)
	if (css == undefined) {
		style?.remove()
		root.removeAttribute(ACCENT_ATTR)
		return
	}
	if (!style) {
		style = document.createElement('style')
		style.id = STYLE_ID
		document.head.appendChild(style)
	}
	style.textContent = css
	root.setAttribute(ACCENT_ATTR, '')
}
