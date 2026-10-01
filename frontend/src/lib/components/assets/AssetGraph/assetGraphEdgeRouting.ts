// Orthogonal edge routing for the asset graph. Every edge is drawn the same way
// — vertical runs joined by horizontal runs with rounded corners — and three
// things keep edges apart where they would otherwise be drawn on top of each
// other:
//
// - Ports: the edges leaving (or entering) one node are spread along its bottom
//   (top) edge, ordered by where their other end lies, instead of all meeting
//   at the node's center.
// - Tracks: the horizontal runs sharing the gap between two layers get distinct
//   heights. Their order follows the channel-routing constraint that removes
//   avoidable crossings: a run must sit above any run that drops out of the gap
//   at an x it spans, and below any run that comes down into the gap at an x it
//   spans.
// - Lanes: edges that skip layers come down a lane beside the nodes in between
//   (chosen by the caller); edges sharing a lane are offset side by side.
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
}

export type RouteEdge = {
	id: string
	source: string
	target: string
	/** Lane x for an edge that must go around the nodes between its endpoints. */
	laneX?: number
}

/** Offsets and heights the edge component combines with its handle coordinates. */
export type EdgeRoute = {
	sourceOffset: number
	targetOffset: number
	/** Height of the edge's run into the target's column (absolute): in the gap
	 * below the source, or for a laned edge in the gap above the target. */
	y: number
	/** For a laned edge: the lane, and the height of the run in the gap below the source. */
	lane?: { x: number; y: number }
}

const PORT_GAP = 14
const LANE_GAP = 10
const MAX_TRACK_GAP = 10
// Clearance kept between a gap's runs and the node rows bounding it.
const GAP_MARGIN_TOP = 18
const GAP_MARGIN_BOTTOM = 14

type Segment = {
	edgeId: string
	gap: number
	from: number
	to: number
	/** x where the run comes down into the gap, and where it leaves it downward. */
	topPin: number
	bottomPin: number
	which: 'upper' | 'lower'
}

export function routeAssetGraphEdges(
	nodes: RouteNode[],
	edges: RouteEdge[],
	layerGap: number
): Map<string, EdgeRoute> {
	const byId = new Map(nodes.map((n) => [n.id, n]))
	const layerTops = [...new Set(nodes.map((n) => n.top))].sort((a, b) => a - b)
	const layerOf = (n: RouteNode) => layerTops.indexOf(n.top)
	// Edges joining the same two nodes (a read and the trigger it derives) share
	// one route and are drawn over each other; one of them stands for the pair.
	const pairKey = (e: RouteEdge) => `${e.source}\u0000${e.target}`
	const representative = new Map<string, RouteEdge>()
	for (const e of edges) {
		const s = byId.get(e.source)
		const t = byId.get(e.target)
		if (s && t && t.top > s.top && !representative.has(pairKey(e))) representative.set(pairKey(e), e)
	}
	const routable = [...representative.values()]

	// Lanes: edges whose lanes land within a few pixels of each other share a
	// corridor; spread them side by side, ordered by their target's x so they
	// leave the corridor without crossing.
	const laneX = new Map<string, number>()
	const laned = routable.filter((e) => e.laneX != undefined).sort((a, b) => a.laneX! - b.laneX!)
	for (let i = 0; i < laned.length; ) {
		let j = i + 1
		while (j < laned.length && laned[j].laneX! - laned[i].laneX! < LANE_GAP) j++
		const group = laned.slice(i, j).sort((a, b) => byId.get(a.target)!.cx - byId.get(b.target)!.cx)
		group.forEach((e, k) => laneX.set(e.id, e.laneX! + (k - (group.length - 1) / 2) * LANE_GAP))
		i = j
	}

	// Ports: where each edge's other end lies, seen from this node.
	const farX = (e: RouteEdge, end: 'source' | 'target') =>
		laneX.get(e.id) ?? byId.get(end === 'source' ? e.target : e.source)!.cx
	const sourceOffset = new Map<string, number>()
	const targetOffset = new Map<string, number>()
	function spread(node: RouteNode, list: RouteEdge[], end: 'source' | 'target') {
		const sorted = [...list].sort((a, b) => farX(a, end) - farX(b, end))
		const gap = Math.min(PORT_GAP, (node.halfW * 1.4) / Math.max(1, sorted.length - 1))
		sorted.forEach((e, k) =>
			(end === 'source' ? sourceOffset : targetOffset).set(
				e.id,
				(k - (sorted.length - 1) / 2) * gap
			)
		)
	}
	for (const n of nodes) {
		spread(
			n,
			routable.filter((e) => e.source === n.id),
			'source'
		)
		spread(
			n,
			routable.filter((e) => e.target === n.id),
			'target'
		)
	}

	// Horizontal runs, keyed by the gap they sit in (gap g lies above layer g).
	const segments: Segment[] = []
	for (const e of routable) {
		const s = byId.get(e.source)!
		const t = byId.get(e.target)!
		const sx = s.cx + sourceOffset.get(e.id)!
		const tx = t.cx + targetOffset.get(e.id)!
		const lx = laneX.get(e.id)
		if (lx == undefined) {
			// Turn in the gap below the source, then down the target's column — the
			// column the caller checked clear of the nodes in between.
			segments.push(seg(e.id, layerOf(s) + 1, sx, tx, 'lower'))
		} else {
			segments.push(seg(e.id, layerOf(s) + 1, sx, lx, 'upper'))
			segments.push(seg(e.id, layerOf(t), lx, tx, 'lower'))
		}
	}

	const heights = new Map<string, number>()
	const byGap = new Map<number, Segment[]>()
	for (const sg of segments) byGap.set(sg.gap, [...(byGap.get(sg.gap) ?? []), sg])
	for (const [gap, list] of byGap) {
		const tracks = assignTracks(list)
		const count = Math.max(...tracks.values()) + 1
		const bandTop = layerTops[gap] - layerGap + GAP_MARGIN_TOP
		const bandBottom = layerTops[gap] - GAP_MARGIN_BOTTOM
		const step = Math.min(MAX_TRACK_GAP, (bandBottom - bandTop) / Math.max(1, count))
		// Centered in the band: a lone run sits midway between the rows.
		const first = (bandTop + bandBottom) / 2 - ((count - 1) * step) / 2
		for (const sg of list) heights.set(`${sg.edgeId}:${sg.which}`, first + tracks.get(sg)! * step)
	}

	const routeOf = new Map<string, EdgeRoute>()
	for (const e of routable) {
		const lx = laneX.get(e.id)
		routeOf.set(pairKey(e), {
			sourceOffset: sourceOffset.get(e.id)!,
			targetOffset: targetOffset.get(e.id)!,
			y: heights.get(`${e.id}:lower`)!,
			...(lx != undefined ? { lane: { x: lx, y: heights.get(`${e.id}:upper`)! } } : {})
		})
	}
	const out = new Map<string, EdgeRoute>()
	for (const e of edges) {
		const r = routeOf.get(pairKey(e))
		if (r) out.set(e.id, r)
	}
	return out
}

function seg(
	edgeId: string,
	gap: number,
	topPin: number,
	bottomPin: number,
	which: Segment['which']
): Segment {
	return {
		edgeId,
		gap,
		from: Math.min(topPin, bottomPin),
		to: Math.max(topPin, bottomPin),
		topPin,
		bottomPin,
		which
	}
}

const spans = (sg: Segment, x: number) => x > sg.from + 0.5 && x < sg.to - 0.5
const overlaps = (a: Segment, b: Segment) => a.from < b.to + PORT_GAP && b.from < a.to + PORT_GAP

/** Track per segment, 0 the highest. */
function assignTracks(list: Segment[]): Map<Segment, number> {
	// below.get(a) = runs that must sit below a.
	const below = new Map<Segment, Set<Segment>>(list.map((s) => [s, new Set()]))
	const indeg = new Map<Segment, number>(list.map((s) => [s, 0]))
	const addAbove = (upper: Segment, lower: Segment) => {
		if (upper === lower || below.get(upper)!.has(lower)) return
		below.get(upper)!.add(lower)
		indeg.set(lower, indeg.get(lower)! + 1)
	}
	for (const a of list) {
		for (const b of list) {
			if (a === b) continue
			// a comes down into the gap where b runs: a's run must be above b's.
			if (spans(b, a.topPin)) addAbove(a, b)
			// a leaves the gap downward where b runs: a's run must be below b's.
			if (spans(b, a.bottomPin)) addAbove(b, a)
		}
	}
	// Topological order; a cycle (two runs each needing to be above the other)
	// cannot be crossing-free, so the least constrained run goes first.
	const order: Segment[] = []
	const done = new Set<Segment>()
	while (order.length < list.length) {
		let next = list.find((s) => !done.has(s) && indeg.get(s) === 0)
		if (!next) {
			for (const s of list)
				if (!done.has(s) && (!next || indeg.get(s)! < indeg.get(next)!)) next = s
		}
		done.add(next!)
		order.push(next!)
		for (const l of below.get(next!)!) indeg.set(l, indeg.get(l)! - 1)
	}
	const track = new Map<Segment, number>()
	for (const s of order) {
		let t = 0
		for (const [upper, lowers] of below) {
			if (lowers.has(s) && track.has(upper)) t = Math.max(t, track.get(upper)! + 1)
		}
		while ([...track].some(([o, ot]) => ot === t && overlaps(o, s))) t++
		track.set(s, t)
	}
	return track
}

/** SVG path through orthogonal points, each corner rounded by up to `radius`. */
export function roundedPath(points: Array<[number, number]>, radius: number): string {
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
		const r = Math.min(radius, inLen / 2, outLen / 2)
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
