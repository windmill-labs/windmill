<script lang="ts">
	import { getContext, onDestroy, onMount } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let id: string
	const { initialized } = getContext<AppViewerContext>('AppViewerContext')

	onMount(() => {
		if (!$initialized.initializedComponents.includes(id)) {
			$initialized.initializedComponents = [...$initialized.initializedComponents, id]
			$initialized = { ...$initialized }
		}
	})

	onDestroy(() => {
		$initialized.initializedComponents = $initialized.initializedComponents.filter((c) => c !== id)
	})
</script>
