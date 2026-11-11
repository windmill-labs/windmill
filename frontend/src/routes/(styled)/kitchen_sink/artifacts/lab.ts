/**
 * The settings /kitchen_sink/artifacts tries on the artifact document. `labCss` turns them into
 * `--doc-*` variables and rules scoped to `.wm-doc`, the hooks `artifactDocument.css` exposes,
 * so the CSS the page prints can be folded into that file as it is.
 */

export type FontKind = 'sans' | 'serif' | 'mono'

export interface LabFont {
	id: string
	label: string
	kind: FontKind
	/** The `font-family` value, fallbacks included. */
	stack: string
	/** Google Fonts `family=` spec. Absent for system fonts and the app's own. */
	google?: string
	/** Shipped with the app and embedded in exports, so no stylesheet loads it. */
	bundled?: boolean
}

const FALLBACK: Record<FontKind, string> = {
	sans: "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
	serif: "ui-serif, Georgia, 'Times New Roman', serif",
	mono: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace'
}

function webFont(id: string, label: string, kind: FontKind, google: string): LabFont {
	return { id, label, kind, stack: `'${label}', 'Noto Color Emoji', ${FALLBACK[kind]}`, google }
}

function systemFont(id: string, label: string, kind: FontKind): LabFont {
	return { id, label, kind, stack: FALLBACK[kind] }
}

/** `stack` is the one `artifactDocument.css` gives the face. */
function bundledFont(id: string, label: string, kind: FontKind, stack: string): LabFont {
	return { id, label, kind, stack, bundled: true }
}

export const FONTS: LabFont[] = [
	bundledFont('inter', 'Inter', 'sans', "Inter, 'Noto Color Emoji', system-ui, sans-serif"),
	webFont(
		'instrument-sans',
		'Instrument Sans',
		'sans',
		'Instrument+Sans:ital,wght@0,400..700;1,400..700'
	),
	webFont('geist', 'Geist', 'sans', 'Geist:wght@400..700'),
	webFont(
		'plex-sans',
		'IBM Plex Sans',
		'sans',
		'IBM+Plex+Sans:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500'
	),
	webFont('source-sans', 'Source Sans 3', 'sans', 'Source+Sans+3:ital,wght@0,400..700;1,400..700'),
	webFont('dm-sans', 'DM Sans', 'sans', 'DM+Sans:ital,opsz,wght@0,9..40,400..700;1,9..40,400..700'),
	webFont('figtree', 'Figtree', 'sans', 'Figtree:ital,wght@0,400..700;1,400..700'),
	webFont('manrope', 'Manrope', 'sans', 'Manrope:wght@400..700'),
	systemFont('system-sans', 'System sans-serif', 'sans'),
	webFont(
		'newsreader',
		'Newsreader',
		'serif',
		'Newsreader:ital,opsz,wght@0,6..72,200..800;1,6..72,200..800'
	),
	webFont(
		'source-serif',
		'Source Serif 4',
		'serif',
		'Source+Serif+4:ital,opsz,wght@0,8..60,400..700;1,8..60,400..700'
	),
	webFont('instrument-serif', 'Instrument Serif', 'serif', 'Instrument+Serif:ital@0;1'),
	webFont(
		'literata',
		'Literata',
		'serif',
		'Literata:ital,opsz,wght@0,7..72,400..700;1,7..72,400..700'
	),
	webFont(
		'fraunces',
		'Fraunces',
		'serif',
		'Fraunces:ital,opsz,wght@0,9..144,400..700;1,9..144,400..700'
	),
	webFont(
		'plex-serif',
		'IBM Plex Serif',
		'serif',
		'IBM+Plex+Serif:ital,wght@0,400;0,500;0,600;0,700;1,400'
	),
	systemFont('system-serif', 'System serif', 'serif'),
	bundledFont(
		'jetbrains-mono',
		'JetBrains Mono',
		'mono',
		"'JetBrains Mono Variable', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace"
	),
	webFont('geist-mono', 'Geist Mono', 'mono', 'Geist+Mono:wght@400..700'),
	webFont('plex-mono', 'IBM Plex Mono', 'mono', 'IBM+Plex+Mono:ital,wght@0,400;0,500;0,600;1,400'),
	webFont(
		'source-code',
		'Source Code Pro',
		'mono',
		'Source+Code+Pro:ital,wght@0,400..700;1,400..700'
	),
	systemFont('system-mono', 'System monospace', 'mono')
]

export function fontById(id: string): LabFont {
	return FONTS.find((f) => f.id === id) ?? FONTS[0]
}

export function googleFontsHref(font: LabFont): string | undefined {
	return font.google && `https://fonts.googleapis.com/css2?family=${font.google}&display=swap`
}

export interface FontPreset {
	id: string
	label: string
	headingFont: string
	bodyFont: string
	monoFont: string
	headingWeight: number
}

export const PRESETS: FontPreset[] = [
	{
		id: 'default',
		label: 'Inter + JetBrains Mono (default)',
		headingFont: 'inter',
		bodyFont: 'inter',
		monoFont: 'jetbrains-mono',
		headingWeight: 600
	},
	{
		id: 'inter-system-mono',
		label: 'Inter + system monospace',
		headingFont: 'inter',
		bodyFont: 'inter',
		monoFont: 'system-mono',
		headingWeight: 600
	},
	{
		id: 'newsreader',
		label: 'Newsreader + Inter',
		headingFont: 'newsreader',
		bodyFont: 'inter',
		monoFont: 'jetbrains-mono',
		headingWeight: 500
	},
	{
		id: 'source',
		label: 'Source Serif + Source Sans',
		headingFont: 'source-serif',
		bodyFont: 'source-sans',
		monoFont: 'source-code',
		headingWeight: 600
	},
	{
		id: 'instrument',
		label: 'Instrument Serif + Instrument Sans',
		headingFont: 'instrument-serif',
		bodyFont: 'instrument-sans',
		monoFont: 'jetbrains-mono',
		headingWeight: 400
	},
	{
		id: 'geist',
		label: 'Geist + Geist Mono',
		headingFont: 'geist',
		bodyFont: 'geist',
		monoFont: 'geist-mono',
		headingWeight: 600
	},
	{
		id: 'plex',
		label: 'IBM Plex Serif + Sans + Mono',
		headingFont: 'plex-serif',
		bodyFont: 'plex-sans',
		monoFont: 'plex-mono',
		headingWeight: 600
	},
	{
		id: 'literata',
		label: 'Literata throughout (book)',
		headingFont: 'literata',
		bodyFont: 'literata',
		monoFont: 'jetbrains-mono',
		headingWeight: 600
	},
	{
		id: 'system',
		label: 'System fonts (nothing to download)',
		headingFont: 'system-sans',
		bodyFont: 'system-sans',
		monoFont: 'system-mono',
		headingWeight: 600
	}
]

/** Sets of Inter's alternate glyphs, named as the font names them. */
export interface InterStyle {
	id: string
	label: string
	subtitle: string
	/** The `font-feature-settings` value. */
	features: string
}

export const INTER_STYLES: InterStyle[] = [
	{ id: 'stock', label: 'Stock', subtitle: 'No alternates', features: 'normal' },
	{
		id: 'windmill',
		label: 'Windmill (default)',
		subtitle: 'ss04 Disambiguation (no slashed zero), cv11 Single-story a',
		features: "'ss04', 'cv11'"
	},
	{
		id: 'slashed',
		label: 'Windmill, slashed zero',
		subtitle: 'ss02 Disambiguation, cv11 Single-story a',
		features: "'ss02', 'cv11'"
	},
	{
		id: 'linear',
		label: 'As Linear sets it',
		subtitle: 'cv01 Alternate one, ss03 Round quotes & commas',
		features: "'cv01', 'ss03'"
	},
	{
		id: 'tailwind',
		label: 'As Tailwind sets it',
		subtitle: 'cv02 cv03 cv04 Open four, six and nine, cv11 Single-story a',
		features: "'cv02', 'cv03', 'cv04', 'cv11'"
	},
	{
		id: 'raycast',
		label: 'As Raycast sets it',
		subtitle: 'ss02 Disambiguation, ss08 Square quotes',
		features: "'ss02', 'ss08'"
	}
]

export type HeadingMarker = 'number' | 'tick' | 'rule' | 'none'
export type TableStyle = 'hairline' | 'zebra' | 'grid'
export type DividerStyle = 'dots' | 'line' | 'mark'
export type CalloutStyle = 'soft' | 'bar'

export interface LabSettings {
	headingFont: string
	bodyFont: string
	monoFont: string
	headingWeight: number
	/** An `INTER_STYLES` id, applied while the body face is Inter. */
	interStyle: string
	/** px */
	size: number
	leading: number
	/** Ratio between heading levels: h3 = size × scale, h1 = size × scale³. */
	scale: number
	/** The heading face's x-height as a share of its font size. */
	headingAdjust: number
	/** Column width, em */
	column: number
	headingMarker: HeadingMarker
	tables: TableStyle
	divider: DividerStyle
	callouts: CalloutStyle
}

const defaults = PRESETS[0]

/**
 * `artifactDocument.css` as it ships. The page stores every setting, so a browser that has
 * opened it keeps the defaults it saw: change `SETTINGS_KEY` along with these.
 */
export const DEFAULT_SETTINGS: LabSettings = {
	headingFont: defaults.headingFont,
	bodyFont: defaults.bodyFont,
	monoFont: defaults.monoFont,
	headingWeight: defaults.headingWeight,
	interStyle: 'windmill',
	size: 15,
	leading: 1.7,
	scale: 1.2,
	headingAdjust: 0.5,
	column: 46,
	headingMarker: 'number',
	tables: 'hairline',
	divider: 'dots',
	callouts: 'soft'
}

export const SETTINGS_KEY = 'wm-artifact-lab-design-v4'

// The Windmill mark as a single-colour mask: the light blades of the logo at lower opacity.
const MARK_MASK = `url("data:image/svg+xml,${encodeURIComponent(
	`<svg xmlns="http://www.w3.org/2000/svg" viewBox="-11.636 -11.636 279.273 279.273"><g opacity=".45"><polygon points="134.78,14.22 114.31,48.21 101.33,69.75 158.22,69.75 177.97,36.95 191.67,14.22"/><polygon points="64.81,131.95 25.15,131.21 0,130.74 28.44,180.01 66.73,180.72 93.26,181.21"/><polygon points="165.38,181.74 184.58,216.46 196.75,238.47 225.19,189.2 206.66,155.69 193.83,132.46"/></g><polygon points="227.55,69.75 186.61,69.75 101.33,69.75 129.78,119.02 158.16,119.02 228.61,119.02 256,119.02"/><polygon points="136.93,132.47 116.46,167.93 73.82,241.78 130.71,241.78 144.9,217.2 180.13,156.18 193.82,132.46"/><polygon points="121.7,131.95 101.23,96.49 58.59,22.63 30.15,71.91 44.34,96.49 79.57,157.5 93.26,181.22"/></svg>`
)}")`

// Each variant restyles `artifactDocument.css`'s default (the first option of each type), so
// that option adds no rule. Selectors repeat the default's own where it would otherwise outrank
// the variant inside the export, which appends these rules at equal specificity.
const HEADING_MARKERS: Record<HeadingMarker, (s: string) => string> = {
	number: () => '',
	tick: (s) => `${s} h2:not(.sr-only)::before {
	content: '';
	display: block;
	width: 1.25em;
	height: 2px;
	margin: 0 0 0.8em;
	border-radius: 1px;
	background: var(--doc-accent);
	opacity: 0.6;
	font-size: 1em;
}`,
	rule: (s) => `${s} h2:not(.sr-only)::before { content: none; }
${s} h2 { padding-bottom: 0.4em; border-bottom: 1px solid var(--doc-rule); }`,
	none: (s) => `${s} h2:not(.sr-only)::before { content: none; }`
}

const TABLES: Record<TableStyle, (s: string) => string> = {
	hairline: () => '',
	zebra: (s) => `${s} td { border-bottom: 0; }
${s} tbody tr:nth-child(even) > td { background: color-mix(in srgb, var(--doc-sunken) 70%, transparent); }`,
	grid: (s) => `${s} :is(th, td) { border-right: 1px solid var(--doc-rule); }
${s} :is(th, td):last-child { border-right: 0; }`
}

const DIVIDERS: Record<DividerStyle, (s: string) => string> = {
	dots: () => '',
	line: (s) => `${s} hr { height: 0; border-top: 1px solid var(--doc-rule); }
${s} hr::after { content: none; }`,
	mark: (s) => `${s} hr::after {
	content: '';
	display: inline-block;
	width: 1.2em;
	height: 1.2em;
	vertical-align: middle;
	background: currentColor;
	-webkit-mask: ${MARK_MASK} center / contain no-repeat;
	mask: ${MARK_MASK} center / contain no-repeat;
}`
}

const CALLOUTS: Record<CalloutStyle, (s: string) => string> = {
	soft: () => '',
	bar: (s) => `${s} .markdown-alert {
	border-left: 3px solid var(--alert);
	border-radius: 0 0.4rem 0.4rem 0;
	background: color-mix(in srgb, var(--alert) 4%, transparent);
}`
}

/** The rules that apply `s` under `scope`, which must select the `.wm-doc` element. */
export function labCss(s: LabSettings, scope: string): string {
	const interStyle = INTER_STYLES.find((x) => x.id === s.interStyle) ?? INTER_STYLES[0]
	const vars = `${scope} {
	--doc-font-heading: ${fontById(s.headingFont).stack};
	--doc-font-body: ${fontById(s.bodyFont).stack};
	--doc-font-mono: ${fontById(s.monoFont).stack};
	--doc-features: ${s.bodyFont === 'inter' ? interStyle.features : 'normal'};
	--doc-heading-weight: ${s.headingWeight};
	--doc-heading-adjust: ${s.headingAdjust};
	--doc-size: ${s.size}px;
	--doc-leading: ${s.leading};
	--doc-scale: ${s.scale};
	--doc-measure: ${s.column}em;
}`
	return [
		vars,
		// The document sets headings and quotes with the body face's features, which another face
		// reads as other glyphs, or as none.
		s.headingFont !== s.bodyFont
			? `${scope} :is(h1, h2, h3, blockquote) { font-feature-settings: normal; }`
			: '',
		HEADING_MARKERS[s.headingMarker](scope),
		TABLES[s.tables](scope),
		DIVIDERS[s.divider](scope),
		CALLOUTS[s.callouts](scope)
	]
		.filter(Boolean)
		.join('\n')
}

/** Stylesheet links for the fonts `s` uses that the app does not ship. */
export function fontLinks(s: LabSettings): string[] {
	const hrefs = [s.headingFont, s.bodyFont, s.monoFont]
		.map((id) => googleFontsHref(fontById(id)))
		.filter((href): href is string => !!href)
	return [...new Set(hrefs)]
}
