<script lang="ts">
	import { useSvelteFlow, type Node } from '@xyflow/svelte'
	import { untrack } from 'svelte'
	import { NODE } from '$lib/components/graph/util'

	// Smoothly pans the viewport so a freshly-created node lands in the middle,
	// keeping the current zoom. Lives inside <SvelteFlow> so useSvelteFlow() has
	// the flow context (same pattern as ViewportResizer). The parent owns the
	// session lifetime — it clears `targetId` once the layout has settled.
	let { targetId, nodes }: { targetId: string | undefined; nodes: Node[] } = $props()

	const { setCenter, getViewport } = useSvelteFlow()

	// Re-centering across reactive settles is intentional: opening the details
	// pane resizes the canvas, which shifts every node's x a tick later. The
	// `lastKey` guard re-fires setCenter only when the target's own center
	// actually moves, so unrelated node updates (activity polls) don't restart
	// the animation.
	let lastKey = ''
	$effect(() => {
		const id = targetId
		const ns = nodes
		if (!id) {
			lastKey = ''
			return
		}
		untrack(() => {
			const node = ns.find((n) => n.id === id)
			if (!node) return // not laid out yet — re-runs when `nodes` updates
			const cx = node.position.x + NODE.width / 2
			const cy = node.position.y + NODE.height / 2
			const key = `${id}:${cx}:${cy}`
			if (key === lastKey) return
			lastKey = key
			const { zoom } = getViewport()
			void setCenter(cx, cy, { zoom, duration: 400 })
		})
	})
</script>
