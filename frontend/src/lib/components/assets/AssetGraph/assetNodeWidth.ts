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
 * `AssetNode` lay out: icon, the widest of kind / title / trigger chip, and the
 * run and script chips. The numbers mirror their Tailwind spacing; change one
 * and the other drifts. Clamped to the regular node width, past which the title
 * truncates as usual.
 */
export function assetsOnlyNodeWidth(n: {
	kind: string
	title: string
	chip: string
	/** The chip leads with an icon (the folded-errors Fix chip). */
	chipIcon?: boolean
	runState: boolean
	/** Producing scripts: one chip for one, two icons and a "+N" for more. */
	scriptChips: number
}): number {
	const icon = 12 + 14 + 10
	const text = Math.max(
		textWidth(n.kind, 0.7),
		textWidth(n.title, 0.75),
		textWidth(n.chip, 0.65) + 12 + 2 + (n.chipIcon ? 14 : 0)
	)
	// The run chip is sized for a two-digit count, so a new run never re-lays the graph.
	const scripts =
		n.scriptChips === 0
			? 0
			: n.scriptChips === 1
				? 20
				: 8 + 2 + 12 * 2 + 2 + (n.scriptChips > 2 ? textWidth(`+${n.scriptChips - 2}`, 0.65) + 2 : 0)
	const trailing = (n.runState ? 12 + 10 + 2 + textWidth('×99', 0.65) + 4 : 0) + scripts + 8
	const width = Math.ceil(icon + text + 4 + trailing + 2 + 8)
	return Math.min(NODE.width, Math.max(MIN_WIDTH, width))
}
