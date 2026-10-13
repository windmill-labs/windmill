/**
 * How the page header gives up width. Whole parts drop in a fixed order, and the bar shows the
 * first stage whose parts, each held at its floor width, fit beside the page's actions. Which
 * stage that is depends on the names in the bar rather than on a pixel breakpoint. Past the last
 * stage the page's actions fold, one level at a time; past their last level the item's name and
 * its summary truncate, then the folders above it.
 */

/** What the breadcrumb itself drops. */
export type TrailStage = {
	/** Inside a fork, the family's name before the fork's. Outside one it is the workspace's own
	 *  name and stays until `wsButton`. */
	familyName: boolean
	/** The item's path. Dropped only for an item that has a summary: the summary is what a
	 *  person calls the thing, while its path is where it is filed. Without one, the path is the
	 *  item's only name and stays. */
	path: boolean
	forkName: boolean
	/** The workspace part as its disc alone, still leading home, beside its picker's chevron. */
	wsButton: boolean
	/** The folders between the first one and the name; collapsed, they read as `…`. */
	middleLevels: boolean
}

export type BarStage = TrailStage & { label: string }

export const BAR_STAGES: BarStage[] = [
	{
		label: 'Full',
		familyName: true,
		path: true,
		forkName: true,
		wsButton: false,
		middleLevels: true
	},
	{
		label: 'Family name dropped',
		familyName: false,
		path: true,
		forkName: true,
		wsButton: false,
		middleLevels: true
	},
	{
		label: 'Path dropped for the summary',
		familyName: false,
		path: false,
		forkName: true,
		wsButton: false,
		middleLevels: true
	},
	{
		label: 'Fork name dropped',
		familyName: false,
		path: false,
		forkName: false,
		wsButton: false,
		middleLevels: true
	},
	{
		label: 'Workspace as its disc',
		familyName: false,
		path: false,
		forkName: false,
		wsButton: true,
		middleLevels: true
	},
	{
		label: 'Middle folders collapsed',
		familyName: false,
		path: false,
		forkName: false,
		wsButton: true,
		middleLevels: false
	}
]

export type Part = 'family' | 'fork' | 'summary' | 'name'

/** The trail at each stage with its names at their floors, and each name's own width. */
export type TrailMeasures = { trail: number[]; family: number; fork: number; name: number }

/** The width a part may truncate to before the next stage drops something whole. The summary's
 *  is wide: below it the path goes rather than the summary losing most of what it says. */
export const FLOOR: Record<Part, number> = { family: 56, fork: 56, summary: 200, name: 160 }
/** Past the last stage and fold, the item's name and then its summary keep this much before the
 *  folders give any. */
export const SQUEEZED_FLOOR = 40

/** Middle folders collapsed to one `…`; the first folder and the name always stay. */
export function shownPathLevels(levels: string[], stage: TrailStage): string[] {
	const scopes = levels.slice(0, -1)
	if (stage.middleLevels || scopes.length <= 1) return levels
	return [scopes[0], '…', levels[levels.length - 1]]
}

/** Whether a part is on screen at a stage. A natural width of 0 means the page has no such part. */
export function partVisible(part: Part, stage: BarStage, natural: Record<Part, number>): boolean {
	if (natural[part] <= 0) return false
	switch (part) {
		case 'summary':
			return true
		case 'family':
			// Outside a fork the family's name is the workspace's: it goes only with the whole part.
			return !stage.wsButton && (stage.familyName || natural.fork <= 0)
		case 'fork':
			return !stage.wsButton && stage.forkName
		case 'name':
			return stage.path || natural.summary <= 0
	}
}

export type BarMeasure = {
	barWidth: number
	/** The bar's padding and the sidebar handle: what is there at every stage. */
	chrome: number
	/** Per stage: the breadcrumb with each name at its floor. */
	trail: number[]
	/** Per stage: the summary at its floor with the separator before it, 0 where there is none. */
	summary: number[]
	/** Per fold level: the actions with the gap before them, undefined for a level not yet seen. */
	actions: (number | undefined)[]
	/** How many levels past the full row the page's actions can fold through. */
	folds: number
	/** How many of those levels fold before the breadcrumb drops anything: buttons that are there
	 *  only while there is room for everything. */
	earlyFolds?: number
	natural: Record<Part, number>
}

export type BarFit = {
	stage: number
	fold: number
	/** Max widths for the parts that give width, in the order they give it. */
	caps: Partial<Record<Part, number>>
	/** Even the name at its squeezed floor does not fit: the folders above it truncate too. */
	scopesGive: boolean
}

/**
 * Flex shrinks every part in proportion, so however lopsided the factors, the name gives a few
 * pixels while the summary still has room, and a few pixels are an ellipsis. The parts are rigid
 * instead and the deficit is taken from them in a strict order, each down to its floor.
 */
export function fitBar(m: BarMeasure): BarFit {
	// A level not yet seen is assumed as narrow as the next one folded further: trying it is how it
	// gets measured, and once it has been, the bar never picks it again unless it fits.
	const actionsAt = (fold: number) => {
		for (let f = fold; f < m.actions.length; f++)
			if (m.actions[f] !== undefined) return m.actions[f]!
		return 0
	}
	const last = BAR_STAGES.length - 1
	const floorWidth = (s: number, f: number) =>
		m.chrome + (m.trail[s] ?? 0) + (m.summary[s] ?? 0) + actionsAt(f)

	const early = Math.min(m.earlyFolds ?? 0, m.folds)
	let stage = last
	let fold = m.folds
	choose: {
		for (let f = 0; f < early; f++) {
			if (floorWidth(0, f) <= m.barWidth) {
				stage = 0
				fold = f
				break choose
			}
		}
		for (let s = 0; s <= last; s++) {
			if (floorWidth(s, early) <= m.barWidth) {
				stage = s
				fold = early
				break choose
			}
		}
		// The other levels fold only once the breadcrumb has given all it can, so a narrowing bar
		// only ever loses things: the parts it dropped do not come back when the buttons fold.
		for (let f = early + 1; f <= m.folds; f++) {
			if (floorWidth(last, f) <= m.barWidth) {
				fold = f
				break choose
			}
		}
	}

	const at = BAR_STAGES[stage]
	// The summary gives last: it is what a person calls the item, so the path's name goes first.
	const parts = (['family', 'fork', 'name', 'summary'] as Part[]).filter((p) =>
		partVisible(p, at, m.natural)
	)
	const slack = (p: Part) => Math.max(0, m.natural[p] - Math.min(m.natural[p], FLOOR[p]))
	// The stage was measured with its parts at their floors; this is its width with them whole.
	let deficit = floorWidth(stage, fold) + parts.reduce((s, p) => s + slack(p), 0) - m.barWidth
	const caps: Partial<Record<Part, number>> = {}
	const take = (p: Part, floor: number) => {
		if (deficit <= 0 || !parts.includes(p)) return
		const current = caps[p] ?? m.natural[p]
		const give = Math.min(deficit, Math.max(0, current - Math.min(m.natural[p], floor)))
		if (give > 0) caps[p] = current - give
		deficit -= give
	}
	for (const p of parts) take(p, FLOOR[p])
	take('name', SQUEEZED_FLOOR)
	take('summary', SQUEEZED_FLOOR)
	return { stage, fold, caps, scopesGive: deficit > 0 }
}
