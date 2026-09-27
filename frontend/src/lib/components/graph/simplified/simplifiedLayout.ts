import type { Edge, Node } from '@xyflow/svelte'
import type { FlowModule } from '$lib/gen'
import { prettyLanguage } from '$lib/common'
import { getAllModules } from '$lib/components/flows/flowExplorer'
import { buildStructureTree, type FlowStructureNode } from '../flowStructure'
import type { FlowGroup } from '../groupEditor.svelte'

/**
 * Left-to-right layout for the simplified flow view. Each layer is a column whose width is its
 * widest node, so a column of short steps stays narrow. Loop, branch and group steps have no node
 * of their own: each is a box around its body or its branches, which stack vertically.
 */

export const SIMPLIFIED = {
	minWidth: 150,
	maxWidth: 340,
	paddingX: 12,
	/** The icon's slot, and the space between it and the text */
	badgeSize: 18,
	badgeGap: 10,
	rowHeight: 16,
	paddingY: 10,
	colGap: 56,
	rowGap: 20,
	loopPad: 14,
	loopHeader: 20,
	/** Column gap where edges split or merge, leaving room around their shared trunk */
	trunkGap: 88,
	/** Column gap holding a junction, with a trunk on each side of it */
	junctionGap: 120,
	/** Extra box side padding and column gap in the editor, where "+" buttons sit there */
	editRing: 12,
	editGap: 32
}

export type SimplifiedNodeKind =
	| 'input'
	| 'result'
	| 'module'
	| 'failure'
	| 'preprocessor'
	| 'empty'
	/** Where two branch groups in a row meet: every exit merges into it, every entry leaves it */
	| 'junction'
	/** A branch's condition or name, above its lane */
	| 'caption'
	/** A "+" between two steps, only laid out when the graph is editable */
	| 'slot'

export type SimplifiedNodeData = {
	kind: SimplifiedNodeKind
	module?: FlowModule
	title: string
	/** Rendered monospace, before `typeLabel` */
	stepId?: string
	typeLabel?: string
	detail?: string
	width: number
	height: number
	/** Set on slots, and on empty-branch dots and junctions once the graph is editable */
	insert?: InsertTarget
	/** On a branch caption, when that branch can be deleted */
	deleteBranch?: { id: string; index: number }
}

/**
 * Where a "+" inserts: the payload `GraphEventHandlers.insert` takes, built the way the full
 * graph's edges build it. `index` counts in the step tree, where a group is a single item.
 */
export type InsertTarget = {
	sourceId: string
	targetId: string
	index: number
	branch?: { rootId: string; branch: number }
	/** Only the flow's first position may take a trigger */
	allowTrigger: boolean
	/** Steps containing this position: a step being moved cannot be pasted inside itself */
	ancestors: string[]
}

export type SimplifiedBoxKind = 'loop' | 'branch' | 'group'

/** A loop, branch or group, drawn as a box around its body or branches */
export type SimplifiedBoxData = {
	kind: SimplifiedBoxKind
	width: number
	height: number
	header: SimplifiedNodeData
	/** Only for groups */
	group?: FlowGroup
}

export type SimplifiedEdgeData = {
	/** Absolute x of the shared trunk a splitting or merging edge turns on; else it bends halfway */
	trunkX?: number
}

export type SimplifiedLayout = { nodes: Node[]; edges: Edge[] }

export type TextMeasurer = (text: string, font: 'title' | 'meta') => number

let canvasCtx: CanvasRenderingContext2D | null | undefined
// Must track the classes SimplifiedNode renders with: text-xs (0.75rem) and text-2xs (0.7rem).
// They are rem-based, so the px size follows the root font size, which is not always 16px.
const FONTS = {
	title: { rem: 0.75, family: '500 {px}px Inter, ui-sans-serif, system-ui, sans-serif' },
	meta: { rem: 0.7, family: '{px}px ui-monospace, SFMono-Regular, Menlo, monospace' }
}

export const measureTextWidth: TextMeasurer = (text, font) => {
	if (canvasCtx === undefined) {
		canvasCtx =
			typeof document !== 'undefined' ? document.createElement('canvas').getContext('2d') : null
	}
	if (!canvasCtx) return text.length * (font === 'title' ? 7 : 6.6)
	const rootPx = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16
	canvasCtx.font = FONTS[font].family.replace('{px}', String(FONTS[font].rem * rootPx))
	return canvasCtx.measureText(text).width
}

function typeLabel(mod: FlowModule): string {
	const v = mod.value
	switch (v.type) {
		case 'rawscript':
			return prettyLanguage(v.language)
		case 'script':
			return v.path.startsWith('hub/') ? 'Hub script' : 'Script'
		case 'flow':
			return 'Subflow'
		case 'forloopflow':
			return 'For loop'
		case 'whileloopflow':
			return 'While loop'
		case 'branchone':
			return 'Branch one'
		case 'branchall':
			return 'Branch all'
		case 'identity':
			return 'Identity'
		case 'aiagent':
			return 'AI agent'
		default:
			return (v as { type: string }).type
	}
}

function moduleTitle(mod: FlowModule, kind: SimplifiedNodeKind): string {
	if (mod.summary) return mod.summary
	const v = mod.value
	switch (v.type) {
		case 'forloopflow':
			return 'For loop'
		case 'whileloopflow':
			return 'While loop'
		case 'branchone':
			return 'Run one branch'
		case 'branchall':
			return 'Run all branches'
		case 'aiagent':
			return v.agent ?? 'AI Agent'
	}
	if (kind === 'preprocessor') return 'Preprocessor'
	if (kind === 'failure') return 'Error handler'
	if ('path' in v && v.path) return v.path
	if (v.type === 'rawscript') return `Inline ${prettyLanguage(v.language)}`
	return 'To be defined'
}

function moduleDetail(mod: FlowModule, title: string): string | undefined {
	const v = mod.value
	const flags = (f: [unknown, string][]) =>
		f
			.filter(([on]) => on)
			.map(([, s]) => s)
			.join(' · ')
	switch (v.type) {
		case 'script':
		case 'flow':
			return v.path !== title ? v.path : undefined
		case 'forloopflow': {
			const iter =
				v.iterator?.type === 'javascript'
					? `over ${v.iterator.expr}`
					: v.iterator?.type === 'static'
						? 'over static value'
						: undefined
			return (
				[
					iter,
					flags([
						[v.parallel, 'parallel'],
						[v.skip_failures, 'skip failures']
					])
				]
					.filter(Boolean)
					.join(' · ') || undefined
			)
		}
		case 'whileloopflow':
			return (
				flags([
					[v.parallel, 'parallel'],
					[v.skip_failures, 'skip failures']
				]) || undefined
			)
		case 'branchone':
			return `${v.branches.length} branch${v.branches.length === 1 ? '' : 'es'} + default`
		case 'branchall':
			return [
				`${v.branches.length} branch${v.branches.length === 1 ? '' : 'es'}`,
				flags([[v.parallel, 'parallel']])
			]
				.filter(Boolean)
				.join(' · ')
		case 'aiagent': {
			const n = v.tools?.length ?? 0
			return n ? `${n} tool${n === 1 ? '' : 's'}` : undefined
		}
	}
	return undefined
}

function sized(
	data: Omit<SimplifiedNodeData, 'width' | 'height'>,
	measure: TextMeasurer
): SimplifiedNodeData {
	const meta = [data.stepId, data.typeLabel].filter(Boolean).join('  ·  ')
	// The error handler and the preprocessor prefix their meta row with an 11px icon and its gap
	const metaIcon = data.kind === 'failure' || data.kind === 'preprocessor' ? 18 : 0
	const content = Math.max(
		measure(data.title, 'title'),
		meta ? measure(meta, 'meta') + metaIcon : 0,
		data.detail ? measure(data.detail, 'meta') : 0
	)
	const width = Math.round(
		Math.min(
			SIMPLIFIED.maxWidth,
			Math.max(
				SIMPLIFIED.minWidth,
				content + SIMPLIFIED.badgeSize + SIMPLIFIED.badgeGap + 2 * SIMPLIFIED.paddingX + 4
			)
		)
	)
	const rows = 1 + (meta ? 1 : 0) + (data.detail ? 1 : 0)
	return { ...data, width, height: rows * SIMPLIFIED.rowHeight + 2 * SIMPLIFIED.paddingY }
}

export function moduleNodeData(
	mod: FlowModule,
	kind: SimplifiedNodeKind,
	measure: TextMeasurer
): SimplifiedNodeData {
	const title = moduleTitle(mod, kind)
	return sized(
		{
			kind,
			module: mod,
			title,
			stepId: mod.id,
			typeLabel: typeLabel(mod).toLowerCase() === title.toLowerCase() ? undefined : typeLabel(mod),
			detail: moduleDetail(mod, title)
		},
		measure
	)
}

// ---- Structured layout ----------------------------------------------------------------------

type Placed = { id: string; data: SimplifiedNodeData; layer: number; y: number }

type Box = {
	id: string
	kind: SimplifiedBoxKind
	header: SimplifiedNodeData
	group?: FlowGroup
	startLayer: number
	endLayer: number
	y: number
	height: number
	children: Box[]
}

/**
 * A "+" whose x is only known once columns are placed: in the gap before a column, or on a box's
 * left or right edge where a lane's edges cross it.
 */
type Slot = {
	id: string
	insert: InsertTarget
	at:
		| { kind: 'gap'; layer: number; where: 'center' | 'beforeTrunk' | 'afterTrunk' }
		| { kind: 'boxLeft' | 'boxRight'; box: string }
	y: number
}

/** A laid-out sub-graph with y relative to its own top. */
type Block = {
	height: number
	nodes: Placed[]
	boxes: Box[]
	edges: Edge[]
	slots: Slot[]
	/** Empty for an empty sequence: the caller wires its predecessors straight through */
	entries: string[]
	exits: string[]
	/** Last layer used, or `layer - 1` if empty */
	endLayer: number
}

type Ctx = { measure: TextMeasurer; modules: Map<string, FlowModule>; editable: boolean }

/** Where a sequence of steps lives in the step tree, as the insert payload names it */
type Container = { rootRef?: string; branch: number; ancestors: string[] }

const EMPTY_BRANCH: SimplifiedNodeData = { kind: 'empty', title: '', width: 10, height: 10 }
const CAPTION_HEIGHT = 16
/** Sits in the gap before its layer's column rather than in the column itself */
const JUNCTION: SimplifiedNodeData = { kind: 'junction', title: '', width: 8, height: 8 }
const SLOT_SIZE = 20

/** The id the insert handler matches a step-tree node by */
const ref = (n: FlowStructureNode) => (n.kind === 'group' ? `group:${n.id}` : n.id)

function shift(b: Block, dy: number): Block {
	return {
		...b,
		nodes: b.nodes.map((n) => ({ ...n, y: n.y + dy })),
		boxes: b.boxes.map((x) => shiftBox(x, dy)),
		slots: b.slots.map((s) => ({ ...s, y: s.y + dy }))
	}
}

function shiftBox(b: Box, dy: number): Box {
	return { ...b, y: b.y + dy, children: b.children.map((c) => shiftBox(c, dy)) }
}

function connect(exits: string[], entries: string[]): Edge[] {
	return exits.flatMap((source) =>
		entries.map((target) => ({
			id: `${source}->${target}`,
			source,
			target,
			type: 'simplifiedEdge'
		}))
	)
}

function leaf(id: string, data: SimplifiedNodeData, layer: number): Block {
	return {
		height: data.height,
		nodes: [{ id, data, layer, y: 0 }],
		boxes: [],
		edges: [],
		slots: [],
		entries: [id],
		exits: [id],
		endLayer: layer
	}
}

function emptyBlock(layer: number): Block {
	return {
		height: 0,
		nodes: [],
		boxes: [],
		edges: [],
		slots: [],
		entries: [],
		exits: [],
		endLayer: layer - 1
	}
}

function insertTarget(
	container: Container,
	index: number,
	sourceId: string,
	targetId: string
): InsertTarget {
	return {
		sourceId,
		targetId,
		index,
		branch: container.rootRef ? { rootId: container.rootRef, branch: container.branch } : undefined,
		allowTrigger: !container.rootRef && index === 0,
		ancestors: container.ancestors
	}
}

/**
 * Chains consecutive blocks, already laid out on consecutive layers, centring them on one line.
 * `insertAt(i)` is the "+" between blocks `i - 1` and `i`, if any.
 */
function joinSequence(blocks: Block[], insertAt: (i: number) => InsertTarget | undefined): Block {
	if (blocks.length === 0) return emptyBlock(0)
	const height = Math.max(...blocks.map((b) => b.height))
	const out: Block = {
		height,
		nodes: [],
		boxes: [],
		edges: [],
		slots: [],
		entries: blocks[0].entries,
		exits: blocks[blocks.length - 1].exits,
		endLayer: blocks[blocks.length - 1].endLayer
	}
	blocks.forEach((b, i) => {
		const s = shift(b, (height - b.height) / 2)
		out.nodes.push(...s.nodes)
		out.boxes.push(...s.boxes)
		out.edges.push(...s.edges)
		out.slots.push(...s.slots)
		if (i === 0) return
		const exits = blocks[i - 1].exits
		const layer = blocks[i - 1].endLayer + 1
		const insert = insertAt(i)
		if (exits.length > 1 && b.entries.length > 1) {
			// Wiring every exit to every entry would draw each lane straight into the one level with
			// it, as if the branches carried on. Merging into one junction first shows they join, and
			// in the editor the junction is also where a step goes between the two.
			const id = `junction:${b.entries[0]}`
			const y = (height - JUNCTION.height) / 2
			out.nodes.push({ id, data: { ...JUNCTION, insert }, layer, y })
			out.edges.push(...connect(exits, [id]), ...connect([id], b.entries))
			return
		}
		out.edges.push(...connect(exits, b.entries))
		if (insert) {
			// Clear of the trunk the edges split or merge on, which sits mid-gap
			const where = b.entries.length > 1 ? 'beforeTrunk' : exits.length > 1 ? 'afterTrunk' : 'center'
			out.slots.push({
				id: `slot:${insert.branch?.rootId ?? ''}:${insert.branch?.branch ?? ''}:${insert.index}`,
				insert,
				at: { kind: 'gap', layer, where },
				y: height / 2
			})
		}
	})
	return out
}

function layoutItems(
	items: FlowStructureNode[],
	layer: number,
	container: Container,
	ctx: Ctx
): Block {
	const blocks: Block[] = []
	let l = layer
	for (const item of items) {
		const b = layoutNode(item, l, container.ancestors, ctx)
		blocks.push(b)
		l = b.endLayer + 1
	}
	if (blocks.length === 0) return emptyBlock(layer)
	return joinSequence(blocks, (i) =>
		ctx.editable ? insertTarget(container, i, ref(items[i - 1]), ref(items[i])) : undefined
	)
}

/**
 * One body or branch of a loop, branch or group: its steps, the "+" at both ends (drawn where
 * the lane crosses the box's edges) and, for a branch, its caption.
 */
function layoutLane(
	items: FlowStructureNode[],
	layer: number,
	container: Container & { rootRef: string },
	boxId: string,
	caption:
		| { id: string; label: string; deleteBranch?: SimplifiedNodeData['deleteBranch'] }
		| undefined,
	ctx: Ctx
): Block {
	const start = `${container.rootRef}-start`
	const end = `${container.rootRef}-end`
	let lane: Block
	if (items.length === 0) {
		// An empty lane gets a placeholder dot so it still reads as a lane of its own
		const insert = ctx.editable ? insertTarget(container, 0, start, end) : undefined
		lane = leaf(
			`${container.rootRef}:empty-${container.branch}`,
			{ ...EMPTY_BRANCH, insert },
			layer
		)
	} else {
		lane = layoutItems(items, layer, container, ctx)
		if (ctx.editable) {
			const edges = [
				{ at: 'boxLeft' as const, insert: insertTarget(container, 0, start, ref(items[0])) },
				{
					at: 'boxRight' as const,
					insert: insertTarget(container, items.length, ref(items[items.length - 1]), end)
				}
			]
			for (const { at, insert } of edges) {
				lane.slots.push({
					id: `slot:${container.rootRef}:${container.branch}:${insert.index}`,
					insert,
					at: { kind: at, box: boxId },
					y: lane.height / 2
				})
			}
		}
	}
	if (!caption) return lane
	const data: SimplifiedNodeData = {
		kind: 'caption',
		title: caption.label,
		deleteBranch: caption.deleteBranch,
		// Stretched to its column's width once known; captions never widen a column.
		width: 0,
		height: CAPTION_HEIGHT - 2
	}
	const s = shift(lane, CAPTION_HEIGHT)
	s.nodes.push({ id: caption.id, data, layer, y: 0 })
	return { ...s, height: lane.height + CAPTION_HEIGHT }
}

/**
 * Wraps `inner` in a box whose header stands for the loop, branch or group itself: it gets no node
 * of its own, and edges run straight into the inner entries and out of its exits.
 */
function boxed(
	box: Pick<Box, 'id' | 'kind' | 'header' | 'group'>,
	layer: number,
	inner: Block
): Block {
	const pad = SIMPLIFIED.loopPad
	const top = pad + SIMPLIFIED.loopHeader
	const height = top + inner.height + pad
	const b = shift(inner, top)
	return {
		...b,
		height,
		boxes: [
			{
				...box,
				startLayer: layer,
				endLayer: inner.endLayer,
				y: 0,
				height,
				children: b.boxes
			}
		]
	}
}

function stackLanes(lanes: Block[]): Block {
	const height = lanes.reduce((a, b) => a + b.height, 0) + SIMPLIFIED.rowGap * (lanes.length - 1)
	const out: Block = {
		height,
		nodes: [],
		boxes: [],
		edges: [],
		slots: [],
		entries: [],
		exits: [],
		endLayer: Math.max(...lanes.map((b) => b.endLayer))
	}
	let y = 0
	for (const lane of lanes) {
		const s = shift(lane, y)
		out.nodes.push(...s.nodes)
		out.boxes.push(...s.boxes)
		out.edges.push(...s.edges)
		out.slots.push(...s.slots)
		out.entries.push(...s.entries)
		out.exits.push(...s.exits)
		y += lane.height + SIMPLIFIED.rowGap
	}
	return out
}

function groupHeader(group: FlowGroup | undefined, stepCount: number): SimplifiedNodeData {
	return {
		kind: 'module',
		title: group?.summary || 'Group',
		detail: `${stepCount} step${stepCount === 1 ? '' : 's'}`,
		width: 0,
		height: 0
	}
}

function layoutNode(node: FlowStructureNode, layer: number, ancestors: string[], ctx: Ctx): Block {
	if (node.kind === 'group') {
		const rootRef = ref(node)
		const container = { rootRef, branch: 0, ancestors }
		const lane = layoutLane(
			node.branches[0]?.children ?? [],
			layer,
			container,
			rootRef,
			undefined,
			ctx
		)
		const header = groupHeader(node.group, node.moduleIds?.length ?? 0)
		return boxed({ id: rootRef, kind: 'group', header, group: node.group }, layer, lane)
	}

	const mod = ctx.modules.get(node.id)
	const self = mod
		? leaf(node.id, moduleNodeData(mod, 'module', ctx.measure), layer)
		: leaf(node.id, sized({ kind: 'module', title: node.id, stepId: node.id }, ctx.measure), layer)
	if (!mod) return self
	const v = mod.value
	const inside = [...ancestors, node.id]

	if (v.type === 'forloopflow' || v.type === 'whileloopflow') {
		const children = node.branches[0]?.children ?? []
		// An empty body stays a plain step when viewing; the editor needs a lane to insert into.
		if (!children.length && !ctx.editable) return self
		const container = { rootRef: node.id, branch: 0, ancestors: inside }
		const lane = layoutLane(children, layer, container, node.id, undefined, ctx)
		const header = moduleNodeData(mod, 'module', ctx.measure)
		return boxed({ id: node.id, kind: 'loop', header }, layer, lane)
	}

	if ((v.type === 'branchone' || v.type === 'branchall') && node.branches.length) {
		// The step tree lists a branch-one's default first; it is shown last, after the conditions.
		const order =
			v.type === 'branchone'
				? [...node.branches.keys()].slice(1).concat(0)
				: [...node.branches.keys()]
		const lanes = order.map((k) => {
			// Branch-all branches all run, so an unnamed one needs no caption outside the editor; a
			// branch-one branch is always captioned with the condition that selects it.
			const label =
				v.type === 'branchone'
					? k === 0
						? 'Default'
						: v.branches[k - 1]?.summary || v.branches[k - 1]?.expr || `Branch ${k}`
					: v.branches[k]?.summary || (ctx.editable ? `Branch ${k + 1}` : undefined)
			const container = { rootRef: node.id, branch: k, ancestors: inside }
			const caption = label
				? {
						id: `${node.id}:caption-${k}`,
						label,
						// A branch-one's default cannot go; `deleteBranch` counts it as branch 0 too.
						deleteBranch:
							ctx.editable && !(v.type === 'branchone' && k === 0)
								? { id: node.id, index: k }
								: undefined
					}
				: undefined
			return layoutLane(node.branches[k].children, layer, container, node.id, caption, ctx)
		})
		const header = moduleNodeData(mod, 'module', ctx.measure)
		return boxed({ id: node.id, kind: 'branch', header }, layer, stackLanes(lanes))
	}

	return self
}

// ---- Global columns ---------------------------------------------------------------------------

type FlatBox = Box & { leftDepth: number; rightDepth: number }

/** Nested boxes sharing a first or last column each need their own padding ring on that side. */
function flattenBoxes(boxes: Box[]): FlatBox[] {
	const out: FlatBox[] = []
	const visit = (b: Box): FlatBox => {
		const children = b.children.map(visit)
		const depth = (side: 'startLayer' | 'endLayer', key: 'leftDepth' | 'rightDepth') =>
			1 + Math.max(0, ...children.filter((c) => c[side] === b[side]).map((c) => c[key]))
		const flat = {
			...b,
			leftDepth: depth('startLayer', 'leftDepth'),
			rightDepth: depth('endLayer', 'rightDepth')
		}
		out.push(flat)
		return flat
	}
	boxes.forEach(visit)
	return out
}

export function layoutSimplifiedFlow(
	opts: {
		modules: FlowModule[]
		/** The step tree with groups resolved; built from `modules` without groups when absent */
		structure?: FlowStructureNode[]
		failureModule?: FlowModule
		preprocessorModule?: FlowModule
		/** Lays out the "+" slots, and a lane for empty loop bodies to insert into */
		editable?: boolean
	},
	measure: TextMeasurer = measureTextWidth
): SimplifiedLayout {
	const ctx: Ctx = {
		measure,
		modules: new Map(getAllModules(opts.modules ?? []).map((m) => [m.id, m])),
		editable: opts.editable ?? false
	}
	const structure = opts.structure ?? buildStructureTree(opts.modules ?? [], [])

	const blocks: Block[] = []
	let layer = 0
	if (opts.preprocessorModule) {
		const data = moduleNodeData(opts.preprocessorModule, 'preprocessor', measure)
		blocks.push(leaf('preprocessor', data, layer++))
	}
	const inputIndex = blocks.length
	const inputData = sized({ kind: 'input', title: 'Input', typeLabel: 'Flow input' }, measure)
	blocks.push(leaf('Input', inputData, layer++))
	const top: Container = { branch: 0, ancestors: [] }
	for (const item of structure) {
		const b = layoutNode(item, layer, [], ctx)
		blocks.push(b)
		layer = b.endLayer + 1
	}
	const resultLayer = layer
	const resultData = sized({ kind: 'result', title: 'Result', typeLabel: 'Flow output' }, measure)
	blocks.push(leaf('Result', resultData, resultLayer))

	const refs = [
		...blocks.slice(0, inputIndex).map(() => 'preprocessor'),
		'Input',
		...structure.map(ref),
		'Result'
	]
	const body = joinSequence(blocks, (i) =>
		ctx.editable && i > inputIndex
			? insertTarget(top, i - inputIndex - 1, refs[i - 1], refs[i])
			: undefined
	)
	const { height, nodes, edges, boxes, slots } = body

	if (opts.failureModule) {
		const data = moduleNodeData(opts.failureModule, 'failure', measure)
		nodes.push({ id: 'failure', data, layer: resultLayer, y: height + SIMPLIFIED.rowGap * 2 })
	}

	// Column widths and x offsets, with room for the loop boxes' padding rings between columns.
	const flatBoxes = flattenBoxes(boxes)
	const layerCount = resultLayer + 1
	const colWidth = new Array(layerCount).fill(0)
	for (const n of nodes) {
		if (n.data.kind !== 'caption' && n.data.kind !== 'junction') {
			colWidth[n.layer] = Math.max(colWidth[n.layer], n.data.width)
		}
	}
	// Where edges split or merge (a branch box has several entries and exits, and two in a row
	// connect every exit to every entry), they all turn on one vertical trunk centred in the gap
	// before the target column. That gap is widened so the trunk stands clear of the boxes on both
	// sides and reads as the point where the edges consolidate.
	const layerOf = new Map(nodes.map((n) => [n.id, n.layer]))
	const count = (key: 'source' | 'target') => {
		const m = new Map<string, number>()
		for (const e of edges) m.set(e[key], (m.get(e[key]) ?? 0) + 1)
		return m
	}
	const outCount = count('source')
	const inCount = count('target')
	const trunkEdges = edges.filter((e) => outCount.get(e.source)! > 1 || inCount.get(e.target)! > 1)
	const trunkLayers = new Set(trunkEdges.map((e) => layerOf.get(e.target)!))
	const junctionLayers = new Set(
		nodes.filter((n) => n.data.kind === 'junction').map((n) => n.layer)
	)

	const colX: number[] = []
	const gapStart: number[] = []
	const gapEnd: number[] = []
	// In the editor a "+" sits on each box edge a lane crosses, and others in the gaps: rings and
	// gaps widen so neighbouring buttons (nested box edges, a merge "+" next to a box) never touch.
	const ring = SIMPLIFIED.loopPad + (ctx.editable ? SIMPLIFIED.editRing : 0)
	const gapExtra = ctx.editable ? SIMPLIFIED.editGap : 0
	let x = 0
	for (let l = 0; l < layerCount; l++) {
		const opening = Math.max(
			0,
			...flatBoxes.filter((b) => b.startLayer === l).map((b) => b.leftDepth)
		)
		if (l > 0) {
			const closing = Math.max(
				0,
				...flatBoxes.filter((b) => b.endLayer === l - 1).map((b) => b.rightDepth)
			)
			gapStart.push(x + closing * ring)
			x +=
				(junctionLayers.has(l)
					? SIMPLIFIED.junctionGap
					: trunkLayers.has(l)
						? SIMPLIFIED.trunkGap
						: SIMPLIFIED.colGap) +
				gapExtra +
				(closing + opening) * ring
			gapEnd.push(x - opening * ring)
		} else {
			gapStart.push(0)
			gapEnd.push(0)
		}
		colX.push(x)
		x += colWidth[l]
	}
	const nodeX = (n: Placed) =>
		n.data.kind === 'junction'
			? (gapStart[n.layer] + gapEnd[n.layer] - n.data.width) / 2
			: colX[n.layer]
	const junctionX = new Map(
		nodes.filter((n) => n.data.kind === 'junction').map((n) => [n.id, nodeX(n)])
	)
	for (const e of trunkEdges) {
		const l = layerOf.get(e.target)!
		// Around a junction, merging edges turn halfway to it and splitting edges halfway from it.
		const into = junctionX.get(e.target)
		const from = junctionX.get(e.source)
		const trunkX =
			into !== undefined
				? (gapStart[l] + into) / 2
				: from !== undefined
					? (from + JUNCTION.width + gapEnd[l]) / 2
					: (gapStart[l] + gapEnd[l]) / 2
		e.data = { trunkX } satisfies SimplifiedEdgeData
	}
	// A caption spans its lane's column, so it truncates only past the widest node there.
	for (const n of nodes) {
		if (n.data.kind === 'caption') n.data = { ...n.data, width: colWidth[n.layer] }
	}

	const xyNodes: Node[] = []
	const boxBounds = new Map<string, { left: number; right: number }>()
	// Outer boxes first so nested ones paint above them.
	const sortedBoxes = [...flatBoxes].sort((a, b) => b.height - a.height)
	for (const b of sortedBoxes) {
		const left = colX[b.startLayer] - b.leftDepth * ring
		const right = colX[b.endLayer] + colWidth[b.endLayer] + b.rightDepth * ring
		boxBounds.set(b.id, { left, right })
		const data: SimplifiedBoxData = {
			kind: b.kind,
			width: right - left,
			height: b.height,
			header: b.header,
			group: b.group
		}
		xyNodes.push({
			id: b.id,
			type: 'simplifiedBox',
			position: { x: left, y: b.y },
			data,
			draggable: false,
			zIndex: -1,
			// Only the group's label is interactive; SimplifiedBox re-enables pointer events on it.
			style: 'pointer-events: none;',
			width: data.width,
			height: data.height
		})
	}
	for (const n of nodes) {
		xyNodes.push({
			id: n.id,
			type: 'simplifiedNode',
			position: { x: nodeX(n), y: n.y },
			data: n.data,
			draggable: false,
			selectable:
				n.data.kind === 'module' ||
				n.data.kind === 'input' ||
				n.data.kind === 'result' ||
				n.data.kind === 'failure' ||
				n.data.kind === 'preprocessor',
			width: n.data.width,
			height: n.data.height
		})
	}
	for (const s of slots) {
		let cx: number
		if (s.at.kind === 'gap') {
			const mid = (gapStart[s.at.layer] + gapEnd[s.at.layer]) / 2
			// Beside the trunk, on the single run before a split or after a merge: as far as possible
			// from the gap's ends, where boxes put their own edge "+"
			const beside = SLOT_SIZE / 2 + 4
			cx =
				s.at.where === 'center'
					? mid
					: s.at.where === 'beforeTrunk'
						? mid - beside
						: mid + beside
		} else {
			const bounds = boxBounds.get(s.at.box)!
			cx = s.at.kind === 'boxLeft' ? bounds.left : bounds.right
		}
		const data: SimplifiedNodeData = {
			kind: 'slot',
			title: '',
			insert: s.insert,
			width: SLOT_SIZE,
			height: SLOT_SIZE
		}
		xyNodes.push({
			id: s.id,
			type: 'simplifiedNode',
			position: { x: cx - SLOT_SIZE / 2, y: s.y - SLOT_SIZE / 2 },
			data,
			draggable: false,
			selectable: false,
			zIndex: 1,
			width: SLOT_SIZE,
			height: SLOT_SIZE
		})
	}
	return { nodes: xyNodes, edges }
}
