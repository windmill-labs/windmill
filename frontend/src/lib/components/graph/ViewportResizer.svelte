<script lang="ts">
	import { useSvelteFlow } from '@xyflow/svelte'
	import { untrack } from 'svelte'
	import { NODE } from './util'

	let { width, nodes, height } = $props()
	const { setViewport, getViewport } = useSvelteFlow()

	$effect(() => {
		;[width]
		untrack(() => onWidthChange(width))
	})
	let lastWidth: number | undefined = undefined

	function onWidthChange(width: number) {
		if (lastWidth === width) return
		const viewport = getViewport()
		let diff = width - (lastWidth ?? 0)
		lastWidth = width
		setViewport({
			...viewport,
			x: viewport.x + diff / 2
		})
	}

	/**
	 * Check if a node is fully visible in the current viewport with margin
	 * @param nodeId - The ID of the node to check
	 * @returns boolean - true if node is fully visible with 20px margin, false otherwise
	 */
	export function isNodeVisible(nodeId: string): boolean {
		const node = nodes.find((n) => n.id === nodeId)
		const viewport = getViewport()
		if (!node || !viewport) return false

		const { x, y, zoom } = viewport
		const nodeX = node.position.x
		const nodeY = node.position.y

		// 20px margin scaled by zoom level
		const margin = 20 / zoom

		// Calculate viewport bounds with margin
		const viewportLeft = -x / zoom + margin
		const viewportTop = -y / zoom + margin
		const viewportRight = viewportLeft + width / zoom - 2 * margin
		const viewportBottom = viewportTop + height / zoom - 2 * margin

		// Calculate node bounds
		const nodeLeft = nodeX
		const nodeTop = nodeY
		const nodeRight = nodeX + NODE.width
		const nodeBottom = nodeY + NODE.height

		// Node is visible only if it's completely within viewport bounds with margin
		return (
			nodeLeft > viewportLeft &&
			nodeRight < viewportRight &&
			nodeTop > viewportTop &&
			nodeBottom < viewportBottom
		)
	}
</script>
