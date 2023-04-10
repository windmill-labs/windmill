<script lang="ts">
	import { getContext, onDestroy, onMount } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let id: string
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	onMount(() => {
		if (!$worldStore.initializedComponents.includes(id)) {
			$worldStore.initializedComponents = [...$worldStore.initializedComponents, id]
		}
	})

	onDestroy(() => {
		$worldStore.initializedComponents = $worldStore.initializedComponents.filter((c) => c !== id)
	})
</script>
