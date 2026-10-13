/**
 * The asset nodes a script can be subscribed to (`// on <asset>`) by dragging:
 * every asset except those downstream of it, whose writes it would then run on
 * in a loop, and those that already trigger it.
 */
export function onDirectiveTargets(
	nodes: Array<{ id: string; type: string }>,
	edges: Array<{ source: string; target: string; kind: string }>,
	scriptId: string
): Set<string> {
	const next = new Map<string, string[]>()
	for (const e of edges) {
		if (e.kind === 'add-anchor') continue
		next.set(e.source, [...(next.get(e.source) ?? []), e.target])
	}
	const downstream = new Set<string>([scriptId])
	const stack = [scriptId]
	while (stack.length) {
		for (const n of next.get(stack.pop()!) ?? []) {
			if (!downstream.has(n)) {
				downstream.add(n)
				stack.push(n)
			}
		}
	}
	const triggering = new Set(
		edges.filter((e) => e.kind === 'trigger-asset' && e.target === scriptId).map((e) => e.source)
	)
	return new Set(
		nodes
			.filter((n) => n.type === 'asset' && !downstream.has(n.id) && !triggering.has(n.id))
			.map((n) => n.id)
	)
}
