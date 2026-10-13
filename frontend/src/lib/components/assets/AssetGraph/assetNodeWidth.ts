import { NODE } from '$lib/components/graph/util'

const MIN_WIDTH = 160

let ctx: CanvasRenderingContext2D | null | undefined

/** Rendered width of `text` at `rem` in the page font, or a rough guess without a DOM. */
function textWidth(text: string, rem: number): number {
	if (typeof document === 'undefined') return text.length * rem * 16 * 0.6
	ctx ??= document.createElement('canvas').getContext('2d')
	const root = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16
	if (!ctx) return text.length * rem * root * 0.6
	const body = getComputedStyle(document.body)
	ctx.font = `${rem * root}px ${body.fontFamily}`
	const spacing = parseFloat(body.letterSpacing) || 0
	return ctx.measureText(text).width + spacing * text.length
}

/**
 * Width of an assets-only asset card, from the pieces `PipelineNodeCard` and
 * `AssetNode` lay out: icon and the widest of kind / title / trigger chip, or the
 * header naming the producing script, if wider. The numbers mirror their Tailwind spacing; change one
 * and the other drifts. Clamped to the regular node width, past which the title
 * truncates as usual.
 */
export function assetsOnlyNodeWidth(n: {
	kind: string
	title: string
	chip: string
	/** The chip leads with an icon (the folded-errors Fix chip). */
	chipIcon?: boolean
	/** The chip ends in the edit icon (an editable trigger). */
	chipEdit?: boolean
	/** The header tab: its text (a script's summary or path, "N scripts"), icon
	 * count, and whether it ends in the menu chevron (else the edit icon) or has a "draft" mark;
	 * undefined without a header. */
	header?: { label: string; icons: number; chevron?: boolean; draft?: boolean }
}): number {
	const icon = 12 + 14 + 10
	const text = Math.max(
		textWidth(n.kind, 0.7),
		textWidth(n.title, 0.75),
		textWidth(n.chip, 0.65) + 12 + 2 + (n.chipIcon ? 14 : 0) + (n.chipEdit ? 4 + 9 : 0)
	)
	const body = Math.ceil(icon + text + 4 + 8 + 2 + 8)
	// Header tab: mx-2 and a 1px border around px-2, then gap-1.5 between its
	// items: the icons, the label, a "draft" mark, and the chevron or edit icon.
	// The run chip is left out: it comes and goes with runs, and a width that
	// followed it would re-lay the graph on a click; the label truncates instead.
	const h = n.header
	const header = h
		? Math.ceil(
				16 +
					2 +
					16 +
					h.icons * (11 + 6) +
					textWidth(h.label, 0.65) +
					(h.draft ? 6 + textWidth('draft', 0.65) : 0) +
					// The menu chevron, or the edit icon of a single script.
					6 +
					(h.chevron ? 11 : 10) +
					4
			)
		: 0
	return Math.min(NODE.width, Math.max(MIN_WIDTH, body, header))
}
