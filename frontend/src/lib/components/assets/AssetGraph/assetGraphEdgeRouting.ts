// Orthogonal edge routing for the asset graph. Every edge is drawn the same way
// — vertical runs joined by horizontal runs with rounded corners — by these
// rules, in order:
//
// 1. Pairs: edges joining the same two nodes (a read and the trigger it
//    derives) share one route and are drawn over each other.
// 2. Bundles: edges of one style into one node gather on a bus and enter it
//    through one trunk (fan-in); edges of one style out of one node leave
//    through one drop and split on a bus (fan-out). Long edges bundle too: those
//    of a fan-in coming down the same lane share it. A shared segment only ever
//    carries edges with the same target (fan-in) or the same source (fan-out),
//    never different sources and different targets, so any merged line still
//    reads back to its ends. An edge belongs to one bundle at most: a long edge
//    prefers its target's, where the long runs are, a short one the larger. A
//    long edge whose lane comes down right over its target, or right beside it,
//    stays out: it enters straight, or by the side, and merging would only add
//    turns.
// 3. Junctions: where lines merge or split, the route carries a point for a dot,
//    so a junction never reads as two lines crossing.
// 4. Style: only edges drawn alike merge, so none hides another.
// 5. Ports: what still leaves (enters) a node separately — single edges and
//    bundles — gets its own point on its bottom (top) edge, as close to where it
//    heads as the node allows: an edge whose ends overlap runs straight.
// 6. Tracks: the horizontal runs sharing the gap between two layers get distinct
//    heights, ordered by the channel-routing constraint that removes avoidable
//    crossings: a run sits above any run that drops out of the gap at an x it
//    spans, and below any run that comes down into the gap at an x it spans.
// 7. Lanes: an edge that would cross a node between its ends comes down a lane
//    beside them (chosen by the caller); lanes shared by unrelated edges sit
//    side by side.
// 8. Sides: an edge may leave its source by a side, at mid-height, or enter its
//    target by one, when that saves a turn, the shortcut crosses no node, and
//    that end of its route is its own (not a bundle's shared drop or trunk). Nodes always sit below what they depend on and
//    the arrow points into the target, so the direction stays plain. A side
//    takes one edge.
//
// Pure: the canvas feeds node boxes and edges, the edge component draws
// `roundedPath` through the points it gets back.

export type RouteNode = {
	id: string
	/** Horizontal center. */
	cx: number
	/** Top edge — nodes of one layer share it. */
	top: number
	halfW: number
	/** Height the layout reserves for it: an upper bound of the rendered one. */
	height: number
}

export type RouteEdge = {
	id: string
	source: string
	target: string
	/** Lane x for an edge that must go around the nodes between its endpoints. */
	laneX?: number
	/** How the edge is drawn: only edges drawn alike merge. */
	style: string
}

/** Orthogonal points from the source handle to the target handle — the first
 * and last y are placeholders the edge replaces with its handles' y — and the
 * junctions on it where it merges with or splits from other edges. */
export type EdgeRoute = {
	points: Array<[number, number]>
	junctions: Array<[number, number]>
	/** The edge leaves its source by this side, at mid-height, instead of its bottom. */
	sourceSide?: 'left' | 'right'
	/** The edge enters its target by this side, at mid-height, instead of its top. */
	targetSide?: 'left' | 'right'
}

const PORT_GAP = 14
const LANE_GAP = 10
const MAX_TRACK_GAP = 10
// Clearance kept between a gap's runs and the node rows bounding it.
const GAP_MARGIN_TOP = 18
const GAP_MARGIN_BOTTOM = 14
// How far from its center, as a share of its half width, a node's ports may sit.
const PORT_SPAN = 0.85
// Least distance between a port's line and an unrelated line beside it.
const AVOID = 8

type Pair = {
	key: string
	source: RouteNode
	target: RouteNode
	laneX?: number
	/** The styles its edges are drawn in: pairs merge only when theirs match. */
	style: string
	edgeIds: string[]
}

/** A horizontal line in one gap, with the x's where lines come down into it and
 * where they leave it downward. */
type Net = { gap: number; topPins: number[]; bottomPins: number[]; from: number; to: number }

export function routeAssetGraphEdges(
	nodes: RouteNode[],
	edges: RouteEdge[],
	layerGap: number
): Map<string, EdgeRoute> {
	const byId = new Map(nodes.map((n) => [n.id, n]))
	const layerTops = [...new Set(nodes.map((n) => n.top))].sort((a, b) => a - b)
	const layerOf = (n: RouteNode) => layerTops.indexOf(n.top)

	// 1. Pairs.
	const pairs = new Map<string, Pair>()
	for (const e of edges) {
		const s = byId.get(e.source)
		const t = byId.get(e.target)
		if (!s || !t || t.top <= s.top) continue
		const key = `${e.source}\u0000${e.target}`
		const p = pairs.get(key)
		if (p) {
			p.edgeIds.push(e.id)
			p.style = [...new Set([...p.style.split('\u0001'), e.style])].sort().join('\u0001')
			if (p.laneX == undefined) p.laneX = e.laneX
		} else {
			pairs.set(key, { key, source: s, target: t, laneX: e.laneX, style: e.style, edgeIds: [e.id] })
		}
	}
	const all = [...pairs.values()]
	const adjacent = (p: Pair) => layerOf(p.target) === layerOf(p.source) + 1

	const SIDE_CLEAR = 6
	const crossesNode = (x1: number, y1: number, x2: number, y2: number, except: RouteNode[]) =>
		nodes.some(
			(n) =>
				!except.includes(n) &&
				Math.min(x1, x2) < n.cx + n.halfW + SIDE_CLEAR &&
				Math.max(x1, x2) > n.cx - n.halfW - SIDE_CLEAR &&
				Math.min(y1, y2) < n.top + n.height + SIDE_CLEAR &&
				Math.max(y1, y2) > n.top - SIDE_CLEAR
		)
	const mid = (n: RouteNode) => n.top + n.height / 2

	// 2. Bundles.
	const outsOf = new Map<string, Pair[]>()
	const insOf = new Map<string, Pair[]>()
	for (const p of all) {
		outsOf.set(p.source.id, [...(outsOf.get(p.source.id) ?? []), p])
		insOf.set(p.target.id, [...(insOf.get(p.target.id) ?? []), p])
	}
	const inKey = (p: Pair) => `in\u0000${p.target.id}\u0000${p.style}`
	const outKey = (p: Pair) => `out\u0000${p.source.id}\u0000${p.style}`
	const bundleOf = new Map<Pair, string>()
	const sizeOf = (key: string) => [...bundleOf.values()].filter((b) => b === key).length
	// Candidates are the pairs sharing an end and a style; a bundle needs two.
	// A long edge whose lane already comes down over its target goes straight
	// in: gathering it with others would only add turns.
	const straightIn = (p: Pair) =>
		!adjacent(p) &&
		p.laneX != undefined &&
		Math.abs(p.laneX - p.target.cx) <= p.target.halfW * PORT_SPAN
	// A long edge whose lane passes right beside its target, with a clear way in,
	// enters by that side: one turn, where a bus would take two. Only the one
	// edge coming that way: a side takes one edge, and several sharing a lane
	// are better merged.
	const sideOf = (p: Pair) => (p.laneX! < p.target.cx ? 'left' : 'right')
	const sideIn = (p: Pair) => {
		if (adjacent(p) || p.laneX == undefined || straightIn(p)) return false
		const t = p.target
		const rivals = insOf
			.get(t.id)!
			.filter((q) => !adjacent(q) && q.laneX != undefined && !straightIn(q) && sideOf(q) === sideOf(p))
		if (rivals.length > 1) return false
		const sideX = sideOf(p) === 'left' ? t.cx - t.halfW : t.cx + t.halfW
		return (
			Math.abs(p.laneX - sideX) <= layerGap &&
			!crossesNode(p.laneX, t.top - layerGap, p.laneX, mid(t), [p.source, t]) &&
			!crossesNode(p.laneX, mid(t), sideX, mid(t), [t])
		)
	}
	const fanInPeers = (p: Pair) =>
		insOf.get(p.target.id)!.filter((q) => q.style === p.style && !straightIn(q) && !sideIn(q))
	const fanOutPeers = (p: Pair) => outsOf.get(p.source.id)!.filter((q) => q.style === p.style)
	for (const p of all) {
		const canIn = !straightIn(p) && !sideIn(p) && fanInPeers(p).length >= 2
		const canOut = fanOutPeers(p).length >= 2
		if (canIn && (!canOut || !adjacent(p) || fanInPeers(p).length >= fanOutPeers(p).length))
			bundleOf.set(p, inKey(p))
		else if (canOut) bundleOf.set(p, outKey(p))
	}
	// A bundle left with one member is just an edge; that edge may still join
	// the other bundle it could have been in. Repeat until nothing moves.
	for (let changed = true; changed; ) {
		changed = false
		for (const [p, b] of [...bundleOf]) {
			if (sizeOf(b) >= 2) continue
			bundleOf.delete(p)
			const other = b === inKey(p) ? outKey(p) : inKey(p)
			const peers = b === inKey(p) ? fanOutPeers(p) : fanInPeers(p)
			if (peers.some((q) => q !== p && bundleOf.get(q) === other)) bundleOf.set(p, other)
			changed = true
		}
	}
	const fanIn = (p: Pair) => bundleOf.get(p)?.startsWith('in\u0000') ?? false
	const fanOut = (p: Pair) => bundleOf.get(p)?.startsWith('out\u0000') ?? false

	// 7. Lanes. One corridor per distinct lane user: a fan-in's members coming
	// down the same lane share it; any other laned edge has its own. Corridors
	// that land within a few pixels of each other are spread side by side,
	// ordered by their target's x so they leave without crossing.
	const corridorKey = (p: Pair) =>
		fanIn(p) ? `${bundleOf.get(p)}\u0000${Math.round(p.laneX! / LANE_GAP)}` : `pair\u0000${p.key}`
	const corridors = new Map<string, { laneX: number; targetX: number }>()
	for (const p of all) {
		if (p.laneX == undefined) continue
		if (!corridors.has(corridorKey(p)))
			corridors.set(corridorKey(p), { laneX: p.laneX, targetX: p.target.cx })
	}
	const corridorX = new Map<string, number>()
	const sorted = [...corridors].sort((a, b) => a[1].laneX - b[1].laneX)
	for (let i = 0; i < sorted.length; ) {
		let j = i + 1
		while (j < sorted.length && sorted[j][1].laneX - sorted[i][1].laneX < LANE_GAP) j++
		const group = sorted.slice(i, j).sort((a, b) => a[1].targetX - b[1].targetX)
		group.forEach(([key, c], k) =>
			corridorX.set(key, c.laneX + (k - (group.length - 1) / 2) * LANE_GAP)
		)
		i = j
	}
	const laneOf = (p: Pair) => (p.laneX == undefined ? undefined : corridorX.get(corridorKey(p)))

	// 5. Ports. Sources are placed first, so an edge from the layer right above
	// enters under its source's port; a longer non-laned edge enters down the
	// target's own column, the one the caller checked clear.
	const outItem = (p: Pair) => (fanOut(p) ? bundleOf.get(p)! : `pair\u0000${p.key}`)
	const inItem = (p: Pair) => (fanIn(p) ? bundleOf.get(p)! : `pair\u0000${p.key}`)
	const portX = new Map<string, number>()
	function place(
		node: RouteNode,
		items: Map<string, number[]>,
		side: 'out' | 'in',
		avoid: (item: string) => number[] = () => []
	) {
		const list = [...items].map(([item, xs]) => ({
			item,
			want: xs.reduce((a, b) => a + b, 0) / xs.length
		}))
		list.sort((a, b) => a.want - b.want)
		const lo = node.cx - node.halfW * PORT_SPAN
		const hi = node.cx + node.halfW * PORT_SPAN
		const gap = Math.min(PORT_GAP, (hi - lo) / Math.max(1, list.length - 1))
		const xs = list.map((e) => Math.min(Math.max(e.want, lo), hi))
		for (let k = 1; k < xs.length; k++) xs[k] = Math.max(xs[k], xs[k - 1] + gap)
		for (let k = xs.length - 1; k >= 0; k--)
			xs[k] = Math.min(xs[k], k === xs.length - 1 ? hi : xs[k + 1] - gap)
		// Step aside from lines that are not this port's but pass where its line
		// would run: drawn over each other, the two would read as one.
		xs.forEach((x, k) => {
			const taken = avoid(list[k].item)
			const clear = (c: number) =>
				c >= lo &&
				c <= hi &&
				taken.every((f) => Math.abs(f - c) >= AVOID) &&
				(k === 0 || c >= xs[k - 1] + gap) &&
				(k === xs.length - 1 || c <= xs[k + 1] - gap)
			if (clear(x)) return
			for (let d = AVOID; d <= node.halfW; d += AVOID / 2) {
				if (clear(x + d)) return void (xs[k] = x + d)
				if (clear(x - d)) return void (xs[k] = x - d)
			}
		})
		list.forEach(({ item }, k) => portX.set(`${side}\u0000${node.id}\u0000${item}`, xs[k]))
	}
	for (const n of nodes) {
		const outs = new Map<string, number[]>()
		for (const p of outsOf.get(n.id) ?? [])
			outs.set(outItem(p), [...(outs.get(outItem(p)) ?? []), laneOf(p) ?? p.target.cx])
		place(n, outs, 'out')
	}
	const sx = (p: Pair) => portX.get(`out\u0000${p.source.id}\u0000${outItem(p)}`)!
	// Lines running down through the gap above a layer: the ports of the layer
	// above and the lanes passing it. A target's own entry may meet those of its
	// own edges, never another's.
	function linesThroughGapAbove(n: RouteNode): Array<{ x: number; pair: Pair }> {
		const L = layerOf(n)
		const lines: Array<{ x: number; pair: Pair }> = []
		for (const p of all) {
			if (layerOf(p.source) === L - 1) lines.push({ x: sx(p), pair: p })
			const lx = laneOf(p)
			if (lx != undefined && layerOf(p.source) + 1 <= L && L <= layerOf(p.target))
				lines.push({ x: lx, pair: p })
		}
		return lines
	}
	for (const n of nodes) {
		const ins = new Map<string, number[]>()
		for (const p of insOf.get(n.id) ?? []) {
			const want = laneOf(p) ?? (adjacent(p) ? sx(p) : p.target.cx)
			ins.set(inItem(p), [...(ins.get(inItem(p)) ?? []), want])
		}
		const through = linesThroughGapAbove(n)
		place(n, ins, 'in', (item) =>
			through
				.filter(({ pair }) => pair.target.id !== n.id || inItem(pair) !== item)
				.map(({ x }) => x)
		)
	}
	const tx = (p: Pair) => portX.get(`in\u0000${p.target.id}\u0000${inItem(p)}`)!
	// Where an edge goes down after its first run: its lane, else the column it
	// enters its target by.
	const descentX = (p: Pair) => laneOf(p) ?? tx(p)

	// Nets: the horizontal lines. A net's key says which edges ride it, and every
	// edge on a shared net has the same source or the same target.
	const nets = new Map<string, Net>()
	type Leg = { key: string; gap: number; from: number; to: number }
	const legsOf = new Map<Pair, Leg[]>()
	for (const p of all) {
		const below = layerOf(p.source) + 1
		const lower = layerOf(p.target)
		const b = bundleOf.get(p)
		const legs: Leg[] = []
		const leg = (key: string, gap: number, from: number, to: number) => {
			const n = nets.get(key) ?? { gap, topPins: [], bottomPins: [], from: Infinity, to: -Infinity }
			n.topPins.push(from)
			n.bottomPins.push(to)
			n.from = Math.min(n.from, from, to)
			n.to = Math.max(n.to, from, to)
			nets.set(key, n)
			legs.push({ key, gap, from, to })
		}
		const lx = laneOf(p)
		if (fanOut(p)) {
			// One drop and one bus for the bundle; a laned member then leaves its
			// lane on its own run.
			leg(b!, below, sx(p), descentX(p))
			if (lx != undefined) leg(`lower\u0000${p.key}`, lower, lx, tx(p))
		} else if (fanIn(p)) {
			if (adjacent(p)) {
				leg(`${b}\u0000bus`, lower, sx(p), tx(p))
			} else {
				// Members from one row heading down one corridor gather first.
				const c = descentX(p)
				leg(`${b}\u0000from\u0000${below}\u0000${Math.round(c)}`, below, sx(p), c)
				if (lx != undefined) leg(`${b}\u0000bus`, lower, c, tx(p))
			}
		} else if (lx != undefined) {
			leg(`upper\u0000${p.key}`, below, sx(p), lx)
			leg(`lower\u0000${p.key}`, lower, lx, tx(p))
		} else {
			// Turn in the gap below the source, then down the target's column.
			leg(`run\u0000${p.key}`, below, sx(p), tx(p))
		}
		legsOf.set(p, legs)
	}

	// 6. Tracks.
	const height = new Map<string, number>()
	const byGap = new Map<number, [string, Net][]>()
	for (const entry of nets) byGap.set(entry[1].gap, [...(byGap.get(entry[1].gap) ?? []), entry])
	for (const [gap, list] of byGap) {
		// A net whose ends meet has no horizontal line to place.
		const runs = list.filter(([, n]) => n.to - n.from > 0.5)
		if (runs.length === 0) continue
		const tracks = assignTracks(runs.map(([, n]) => n))
		const count = Math.max(...tracks) + 1
		const bandTop = layerTops[gap] - layerGap + GAP_MARGIN_TOP
		const bandBottom = layerTops[gap] - GAP_MARGIN_BOTTOM
		const step = Math.min(MAX_TRACK_GAP, (bandBottom - bandTop) / Math.max(1, count))
		// Centered in the band: a lone run sits midway between the rows.
		const first = (bandTop + bandBottom) / 2 - ((count - 1) * step) / 2
		runs.forEach(([key], i) => height.set(key, first + tracks[i] * step))
	}
	const yOf = (leg: Leg) =>
		height.get(leg.key) ?? layerTops[leg.gap] - (layerGap - GAP_MARGIN_TOP + GAP_MARGIN_BOTTOM) / 2

	// 3. Junctions: a point on a net where three or more line directions meet —
	// a line coming down into it, one leaving it downward, a run left, a run right.
	function junctionsOf(n: Net): number[] {
		const at = (pins: number[], x: number) => pins.some((p) => Math.abs(p - x) < 0.5)
		return [...new Set([...n.topPins, ...n.bottomPins])].filter(
			(x) =>
				(at(n.topPins, x) ? 1 : 0) +
					(at(n.bottomPins, x) ? 1 : 0) +
					(x > n.from + 0.5 ? 1 : 0) +
					(x < n.to - 0.5 ? 1 : 0) >=
				3
		)
	}

	// 8. Sides.
	const sideTaken = new Set<string>()
	type Shortcut = { points: Array<[number, number]>; side: 'left' | 'right' }
	// Come down beside the target to its mid-height and go in sideways, instead
	// of turning into the gap above it and down into its top.
	function enterBySide(pts: Array<[number, number]>, p: Pair): Shortcut | undefined {
		const n = pts.length
		if (n < 4) return undefined
		const [x, y] = pts[n - 3]
		if (pts[n - 2][1] !== y || pts[n - 4][0] !== x) return undefined
		const t = p.target
		const side = x < t.cx - t.halfW - SIDE_CLEAR ? 'left' : x > t.cx + t.halfW + SIDE_CLEAR ? 'right' : undefined
		if (!side || sideTaken.has(`${t.id}:${side}`)) return undefined
		const sideX = side === 'left' ? t.cx - t.halfW : t.cx + t.halfW
		if (crossesNode(x, y, x, mid(t), [p.source, t]) || crossesNode(x, mid(t), sideX, mid(t), [t]))
			return undefined
		return { points: [...pts.slice(0, n - 3), [x, mid(t)], [sideX, mid(t)]], side }
	}
	// Leave the source sideways at its mid-height straight to where the edge goes
	// down, instead of dropping into the gap below it and turning there.
	function leaveBySide(pts: Array<[number, number]>, p: Pair): Shortcut | undefined {
		if (pts.length < 4) return undefined
		const [x, y] = pts[2]
		if (pts[1][1] !== y || pts[3][0] !== x) return undefined
		const s = p.source
		const side = x < s.cx - s.halfW - SIDE_CLEAR ? 'left' : x > s.cx + s.halfW + SIDE_CLEAR ? 'right' : undefined
		if (!side || sideTaken.has(`${s.id}:${side}`)) return undefined
		const sideX = side === 'left' ? s.cx - s.halfW : s.cx + s.halfW
		if (crossesNode(sideX, mid(s), x, mid(s), [s]) || crossesNode(x, mid(s), x, y, [s, p.target]))
			return undefined
		return { points: [[sideX, mid(s)], [x, mid(s)], ...pts.slice(3)], side }
	}
	const length = (pts: Array<[number, number]>) =>
		pts.slice(1).reduce((a, q, i) => a + Math.abs(q[0] - pts[i][0]) + Math.abs(q[1] - pts[i][1]), 0)

	const out = new Map<string, EdgeRoute>()
	for (const p of all) {
		let points: Array<[number, number]> = [[sx(p), p.source.top]]
		const junctions: Array<[number, number]> = []
		for (const leg of legsOf.get(p)!) {
			const y = yOf(leg)
			points.push([leg.from, y], [leg.to, y])
			// The junctions on this edge's own path: where it joins or leaves the net.
			for (const x of junctionsOf(nets.get(leg.key)!))
				if (Math.abs(x - leg.from) < 0.5 || Math.abs(x - leg.to) < 0.5) junctions.push([x, y])
		}
		points.push([tx(p), p.target.top])
		points = dropCollinear(points)
		let sourceSide: 'left' | 'right' | undefined
		let targetSide: 'left' | 'right' | undefined
		// Only an end the edge has to itself may move to a side: a fan-out member
		// owns its way into its target only past its own lane (short of it, the
		// line it would come down is the bundle's drop); a fan-in member owns
		// neither end once it joins the others.
		if (!fanIn(p)) {
			// A route with a turn at each end (a laned one) may shortcut both; a
			// single run takes whichever shortcut is shorter.
			const enter = fanOut(p) && laneOf(p) == undefined ? undefined : enterBySide(points, p)
			const leave = bundleOf.has(p) ? undefined : leaveBySide(points, p)
			const both = points.length >= 6
			if (enter && (both || !leave || length(enter.points) <= length(leave.points))) {
				points = enter.points
				targetSide = enter.side
				sideTaken.add(`${p.target.id}:${enter.side}`)
			}
			if (leave && (both || !targetSide)) {
				const again = leaveBySide(points, p)
				if (again) {
					points = again.points
					sourceSide = again.side
					sideTaken.add(`${p.source.id}:${again.side}`)
				}
			}
		}
		for (const id of p.edgeIds)
			out.set(id, {
				points,
				junctions,
				...(sourceSide ? { sourceSide } : {}),
				...(targetSide ? { targetSide } : {})
			})
	}
	return out
}

/**
 * Lane for an edge whose way down its target's column is blocked by a node in
 * the rows between its ends: the nearest x, to the left or to the right, that
 * clears every node in those rows — not only the first one in the way.
 * undefined when the column is clear.
 */
export function detourLane(
	source: RouteNode,
	target: RouteNode,
	nodes: RouteNode[],
	pad: number
): number | undefined {
	if (target.top <= source.top) return undefined
	const between = nodes.filter(
		(n) => n !== source && n !== target && n.top > source.top && n.top < target.top
	)
	const hits = (x: number) => between.filter((n) => Math.abs(x - n.cx) < n.halfW + 8)
	if (hits(target.cx).length === 0) return undefined
	// Step past whatever the lane still hits until nothing does; each step moves
	// strictly outward, so it ends.
	const clearToward = (dir: -1 | 1) => {
		let x = target.cx
		for (let h = hits(x); h.length > 0; h = hits(x))
			x =
				dir < 0
					? Math.min(...h.map((n) => n.cx - n.halfW - pad))
					: Math.max(...h.map((n) => n.cx + n.halfW + pad))
		return x
	}
	const left = clearToward(-1)
	const right = clearToward(1)
	const detour = (x: number) => Math.abs(x - source.cx) + Math.abs(x - target.cx)
	return detour(left) <= detour(right) ? left : right
}

/** Track per net, 0 the highest: the order of the lines from top to bottom is
 * the one with the fewest crossings found, then each line takes the highest
 * track below every earlier line it overlaps. */
function assignTracks(list: Net[]): number[] {
	const spans = (n: Net, x: number) => x > n.from + 0.5 && x < n.to - 0.5
	const overlaps = (a: Net, b: Net) => a.from < b.to + PORT_GAP && b.from < a.to + PORT_GAP
	// Crossings when a sits above b: a's lines leaving the gap pass through b,
	// and b's lines coming into it pass through a.
	const cost = list.map((a) =>
		list.map((b) =>
			a === b
				? 0
				: a.bottomPins.filter((x) => spans(b, x)).length +
					b.topPins.filter((x) => spans(a, x)).length
		)
	)
	const total = (order: number[]) => {
		let sum = 0
		for (let i = 0; i < order.length; i++)
			for (let j = i + 1; j < order.length; j++) sum += cost[order[i]][order[j]]
		return sum
	}
	// Start from the order each pair prefers on its own, then move one line at a
	// time to wherever it removes crossings, until no move helps.
	let order = list
		.map((_, i) => i)
		.sort((i, j) => cost[i][j] - cost[j][i] || list[i].from - list[j].from)
	for (let improved = true; improved; ) {
		improved = false
		for (let k = 0; k < order.length; k++) {
			const rest = order.filter((_, i) => i !== k)
			let best = order
			let bestCost = total(order)
			for (let pos = 0; pos <= rest.length; pos++) {
				const tried = [...rest.slice(0, pos), order[k], ...rest.slice(pos)]
				const c = total(tried)
				if (c < bestCost) {
					best = tried
					bestCost = c
				}
			}
			if (best !== order) {
				order = best
				improved = true
			}
		}
	}
	const track: number[] = list.map(() => -1)
	order.forEach((i, k) => {
		let t = 0
		for (const j of order.slice(0, k)) if (overlaps(list[i], list[j])) t = Math.max(t, track[j] + 1)
		track[i] = t
	})
	return track
}

/** Drop repeated points and the middle of three on one line, so no turn of zero
 * width is drawn. */
function dropCollinear(points: Array<[number, number]>): Array<[number, number]> {
	const out: Array<[number, number]> = []
	for (const p of points) {
		const prev = out[out.length - 1]
		if (prev && prev[0] === p[0] && prev[1] === p[1]) continue
		const prev2 = out[out.length - 2]
		if (
			prev &&
			prev2 &&
			((prev2[0] === prev[0] && prev[0] === p[0]) || (prev2[1] === prev[1] && prev[1] === p[1]))
		) {
			out[out.length - 1] = p
			continue
		}
		out.push(p)
	}
	return out
}

/** SVG path through orthogonal points, each corner rounded by up to `radius`
 * but those in `sharp`. */
export function roundedPath(
	points: Array<[number, number]>,
	radius: number,
	/** Corners drawn sharp: junctions, where other lines meet this one exactly. */
	sharp: Array<[number, number]> = []
): string {
	const pts = points.filter(
		(p, i) => i === 0 || p[0] !== points[i - 1][0] || p[1] !== points[i - 1][1]
	)
	if (pts.length < 2) return ''
	let d = `M ${pts[0][0]} ${pts[0][1]}`
	for (let i = 1; i < pts.length - 1; i++) {
		const [px, py] = pts[i - 1]
		const [cx, cy] = pts[i]
		const [nx, ny] = pts[i + 1]
		const inLen = Math.hypot(cx - px, cy - py)
		const outLen = Math.hypot(nx - cx, ny - cy)
		const isSharp = sharp.some(([sx, sy]) => Math.abs(sx - cx) < 0.5 && Math.abs(sy - cy) < 0.5)
		const r = isSharp ? 0 : Math.min(radius, inLen / 2, outLen / 2)
		const ax = cx - ((cx - px) / inLen) * r
		const ay = cy - ((cy - py) / inLen) * r
		const bx = cx + ((nx - cx) / outLen) * r
		const by = cy + ((ny - cy) / outLen) * r
		d += ` L ${ax} ${ay} Q ${cx} ${cy} ${bx} ${by}`
	}
	const last = pts[pts.length - 1]
	return `${d} L ${last[0]} ${last[1]}`
}

/** The point halfway along a polyline, where an edge's badges sit. */
export function polylineMidpoint(points: Array<[number, number]>): [number, number] {
	const lens = points.slice(1).map((p, i) => Math.hypot(p[0] - points[i][0], p[1] - points[i][1]))
	let half = lens.reduce((a, b) => a + b, 0) / 2
	for (let i = 0; i < lens.length; i++) {
		if (half <= lens[i] && lens[i] > 0) {
			const [ax, ay] = points[i]
			const [bx, by] = points[i + 1]
			return [ax + ((bx - ax) * half) / lens[i], ay + ((by - ay) * half) / lens[i]]
		}
		half -= lens[i]
	}
	return points[points.length - 1]
}
