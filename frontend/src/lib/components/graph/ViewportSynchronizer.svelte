<script lang="ts">
	import { useSvelteFlow, type Viewport } from '@xyflow/svelte'
	import { untrack } from 'svelte'

	interface Props {
		sharedViewport: Viewport
		onLocalChange: (viewport: Viewport, isUserInitiated: boolean) => void
	}

	let { sharedViewport, onLocalChange }: Props = $props()
	const { setViewport, getViewport } = useSvelteFlow()

	let isApplyingSharedChange = false

	// Watch for shared viewport changes and apply them locally
	$effect(() => {
		sharedViewport.x, sharedViewport.y, sharedViewport.zoom
		untrack(() => {
			if (!isApplyingSharedChange) {
				setViewport(sharedViewport, { duration: 0 })
			}
		})
	})

	// Export function to be called when local viewport changes (from onmove)
	export function handleLocalViewportChange(event: MouseEvent | TouchEvent | null, viewport: Viewport) {
		// Only propagate user-initiated changes (not programmatic ones)
		const isUserInitiated = event !== null
		if (isUserInitiated) {
			isApplyingSharedChange = true
			onLocalChange(viewport, isUserInitiated)
			// Reset flag after a tick to allow the other graph to update
			setTimeout(() => {
				isApplyingSharedChange = false
			}, 0)
		}
	}
</script>
