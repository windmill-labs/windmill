<script lang="ts">
	import { useSvelteFlow, type Viewport } from '@xyflow/svelte'
	import { tick, untrack } from 'svelte'

	interface Props {
		sharedViewport: Viewport
		onLocalChange: (viewport: Viewport, isUserInitiated: boolean) => void
	}

	let { sharedViewport, onLocalChange }: Props = $props()
	const { setViewport, getViewport } = useSvelteFlow()

	let isApplyingSharedChange = false

	// Watch for shared viewport changes and apply them locally
	$effect(() => {
		;(sharedViewport.x, sharedViewport.y, sharedViewport.zoom)
		untrack(() => {
			if (!isApplyingSharedChange) {
				setViewport(sharedViewport, { duration: 0 })
			}
		})
	})

	// Export function to be called when local viewport changes (from onmove)
	export async function handleLocalViewportChange(
		event: MouseEvent | TouchEvent | null,
		viewport: Viewport
	) {
		// Only propagate user-initiated changes (not programmatic ones)
		const isUserInitiated = event !== null
		if (isUserInitiated) {
			isApplyingSharedChange = true
			onLocalChange(viewport, isUserInitiated)
			await tick()
			isApplyingSharedChange = false
		}
	}

	export async function zoomIn() {
		const viewport = getViewport()
		const newZoom = Math.min(viewport.zoom + 0.1, 1.2)
		setViewport({ ...viewport, zoom: newZoom })
		await tick()
		const updatedViewport = getViewport()
		isApplyingSharedChange = true
		onLocalChange(updatedViewport, false)
		await tick()
		isApplyingSharedChange = false
	}

	export async function zoomOut() {
		const viewport = getViewport()
		const newZoom = Math.max(viewport.zoom - 0.1, 0.2)
		setViewport({ ...viewport, zoom: newZoom })
		await tick()
		const updatedViewport = getViewport()
		isApplyingSharedChange = true
		onLocalChange(updatedViewport, false)
		await tick()
		isApplyingSharedChange = false
	}
</script>
