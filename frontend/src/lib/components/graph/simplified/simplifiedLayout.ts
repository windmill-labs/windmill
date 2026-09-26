import type { Edge, Node } from '@xyflow/svelte'
import type { FlowModule } from '$lib/gen'
import { prettyLanguage } from '$lib/common'

/**
 * Left-to-right layout for the simplified flow view. Each layer is a column whose width is its
 * widest node, so a column of short steps stays narrow. Loop and branch steps have no node of
 * their own: each is a box around its body or its branches, which stack vertically.
 */

export const SIMPLIFIED = {
	minWidth: 150,
	maxWidth: 340,
	paddingX: 12,
	/** The icon's badge, and the space between it and the text */
	badgeSize: 24,
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
	junctionGap: 120
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
}

/** A loop or branch step, drawn as a box around its body or branches */
export type SimplifiedBoxData = { width: number; height: number; header: SimplifiedNodeData }

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
			Math.max(SIMPLIFIED.minWidth, content + SIMPLIFIED.badgeSize + SIMPLIFIED.badgeGap + 2 * SIMPLIFIED.paddingX + 4)
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
			typeLabel:
				typeLabel(mod).toLowerCase() === title.toLowerCase() ? undefined : typeLabel(mod),
			detail: moduleDetail(mod, title)
		},
		measure
	)
}

// ---- Structured layout ----------------------------------------------------------------------

type Placed = { id: string; data: SimplifiedNodeData; layer: number; y: number }

type Box = {
	id: string
	header: SimplifiedNodeData
	startLayer: number
	endLayer: number
	y: number
	height: number
	children: Box[]
}

/** A laid-out sub-graph with y relative to its own top. */
type Block = {
	height: number
	nodes: Placed[]
	boxes: Box[]
	edges: Edge[]
	/** Empty for an empty sequence: the caller wires its predecessors straight through */
	entries: string[]
	exits: string[]
	/** Last layer used, or `layer - 1` if empty */
	endLayer: number
}

const EMPTY_BRANCH: SimplifiedNodeData = { kind: 'empty', title: '', width: 10, height: 10 }
const CAPTION_HEIGHT = 16
/** Sits in the gap before its layer's column rather than in the column itself */
const JUNCTION: SimplifiedNodeData = { kind: 'junction', title: '', width: 8, height: 8 }

function shift(b: Block, dy: number): Block {
	return {
		...b,
		nodes: b.nodes.map((n) => ({ ...n, y: n.y + dy })),
		boxes: b.boxes.map((x) => shiftBox(x, dy))
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
		entries: [id],
		exits: [id],
		endLayer: layer
	}
}

function layoutSequence(modules: FlowModule[], layer: number, measure: TextMeasurer): Block {
	const blocks: Block[] = []
	let l = layer
	for (const mod of modules) {
		const b = layoutModule(mod, l, measure)
		blocks.push(b)
		l = b.endLayer + 1
	}
	if (blocks.length === 0) {
		return {
			height: 0,
			nodes: [],
			boxes: [],
			edges: [],
			entries: [],
			exits: [],
			endLayer: layer - 1
		}
	}
	const height = Math.max(...blocks.map((b) => b.height))
	const out: Block = {
		height,
		nodes: [],
		boxes: [],
		edges: [],
		entries: blocks[0].entries,
		exits: blocks[blocks.length - 1].exits,
		endLayer: blocks[blocks.length - 1].endLayer
	}
	blocks.forEach((b, i) => {
		const s = shift(b, (height - b.height) / 2)
		out.nodes.push(...s.nodes)
		out.boxes.push(...s.boxes)
		out.edges.push(...s.edges)
		if (i === 0) return
		const exits = blocks[i - 1].exits
		if (exits.length > 1 && b.entries.length > 1) {
			// Wiring every exit to every entry would draw each lane straight into the one level with
			// it, as if the branches carried on. Merging into one junction first shows they join.
			const id = `junction:${b.entries[0]}`
			const y = (height - JUNCTION.height) / 2
			out.nodes.push({ id, data: JUNCTION, layer: blocks[i - 1].endLayer + 1, y })
			out.edges.push(...connect(exits, [id]), ...connect([id], b.entries))
		} else {
			out.edges.push(...connect(exits, b.entries))
		}
	})
	return out
}

/**
 * Wraps `inner` in a box whose header stands for the loop or branch step itself: that step gets no
 * node of its own, and edges run straight into the inner entries and out of its exits.
 */
function boxed(mod: FlowModule, layer: number, inner: Block, measure: TextMeasurer): Block {
	const pad = SIMPLIFIED.loopPad
	const top = pad + SIMPLIFIED.loopHeader
	const height = top + inner.height + pad
	const b = shift(inner, top)
	return {
		...b,
		height,
		boxes: [
			{
				id: mod.id,
				header: moduleNodeData(mod, 'module', measure),
				startLayer: layer,
				endLayer: inner.endLayer,
				y: 0,
				height,
				children: b.boxes
			}
		]
	}
}

function layoutModule(mod: FlowModule, layer: number, measure: TextMeasurer): Block {
	const v = mod.value

	if ((v.type === 'forloopflow' || v.type === 'whileloopflow') && v.modules?.length) {
		return boxed(mod, layer, layoutSequence(v.modules, layer, measure), measure)
	}

	if (
		(v.type === 'branchone' || v.type === 'branchall') &&
		(v.type === 'branchone' || v.branches.length)
	) {
		// Branch-all branches all run, so an unnamed one needs no caption; a branch-one branch is
		// always captioned with the condition that selects it.
		const branches: { label?: string; modules: FlowModule[] }[] = v.branches.map((b, i) => ({
			label: b.summary || (v.type === 'branchone' ? b.expr || `Branch ${i + 1}` : undefined),
			modules: b.modules ?? []
		}))
		if (v.type === 'branchone') branches.push({ label: 'Default', modules: v.default ?? [] })

		// An empty branch gets a placeholder dot so it still reads as a lane of its own.
		const lanes = branches.map((b, i) => {
			const seq = b.modules.length
				? layoutSequence(b.modules, layer, measure)
				: leaf(`${mod.id}:empty-${i}`, EMPTY_BRANCH, layer)
			if (!b.label) return seq
			const caption: SimplifiedNodeData = {
				kind: 'caption',
				title: b.label,
				// Stretched to its column's width once known; captions never widen a column.
				width: 0,
				height: CAPTION_HEIGHT - 2
			}
			const s = shift(seq, CAPTION_HEIGHT)
			s.nodes.push({ id: `${mod.id}:caption-${i}`, data: caption, layer, y: 0 })
			return { ...s, height: seq.height + CAPTION_HEIGHT }
		})
		const height = lanes.reduce((a, b) => a + b.height, 0) + SIMPLIFIED.rowGap * (lanes.length - 1)
		const inner: Block = {
			height,
			nodes: [],
			boxes: [],
			edges: [],
			entries: [],
			exits: [],
			endLayer: Math.max(...lanes.map((b) => b.endLayer))
		}
		let y = 0
		for (const lane of lanes) {
			const s = shift(lane, y)
			inner.nodes.push(...s.nodes)
			inner.boxes.push(...s.boxes)
			inner.edges.push(...s.edges)
			inner.entries.push(...s.entries)
			inner.exits.push(...s.exits)
			y += lane.height + SIMPLIFIED.rowGap
		}
		return boxed(mod, layer, inner, measure)
	}

	return leaf(mod.id, moduleNodeData(mod, 'module', measure), layer)
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
		failureModule?: FlowModule
		preprocessorModule?: FlowModule
	},
	measure: TextMeasurer = measureTextWidth
): SimplifiedLayout {
	let layer = 0
	const prefix: Block[] = []
	if (opts.preprocessorModule) {
		prefix.push(
			leaf(
				'preprocessor',
				moduleNodeData(opts.preprocessorModule, 'preprocessor', measure),
				layer++
			)
		)
	}
	const inputData = sized({ kind: 'input', title: 'Input', typeLabel: 'Flow input' }, measure)
	prefix.push(leaf('Input', inputData, layer++))

	const body = layoutSequence(opts.modules ?? [], layer, measure)
	const resultLayer = body.endLayer + 1
	const result = leaf(
		'Result',
		sized({ kind: 'result', title: 'Result', typeLabel: 'Flow output' }, measure),
		resultLayer
	)

	const height = Math.max(body.height, ...prefix.map((b) => b.height), result.height)
	const center = (b: Block) => shift(b, (height - b.height) / 2)

	const nodes: Placed[] = []
	const edges: Edge[] = []
	const boxes: Box[] = []
	let exits: string[] = []
	for (const b of [...prefix.map(center), center(body), center(result)]) {
		nodes.push(...b.nodes)
		edges.push(...b.edges)
		boxes.push(...b.boxes)
		if (b.entries.length) {
			edges.push(...connect(exits, b.entries))
			exits = b.exits
		}
	}

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
			gapStart.push(x + closing * SIMPLIFIED.loopPad)
			x +=
				(junctionLayers.has(l)
					? SIMPLIFIED.junctionGap
					: trunkLayers.has(l)
						? SIMPLIFIED.trunkGap
						: SIMPLIFIED.colGap) +
				(closing + opening) * SIMPLIFIED.loopPad
			gapEnd.push(x - opening * SIMPLIFIED.loopPad)
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
	// Outer boxes first so nested ones paint above them.
	const sortedBoxes = [...flatBoxes].sort((a, b) => b.height - a.height)
	for (const b of sortedBoxes) {
		const left = colX[b.startLayer] - b.leftDepth * SIMPLIFIED.loopPad
		const right = colX[b.endLayer] + colWidth[b.endLayer] + b.rightDepth * SIMPLIFIED.loopPad
		const data: SimplifiedBoxData = { width: right - left, height: b.height, header: b.header }
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
	return { nodes: xyNodes, edges }
}
