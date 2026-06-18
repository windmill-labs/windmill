<script lang="ts">
	import { getContext, onDestroy, onMount } from 'svelte'
	import type { AppViewerContext } from '../../types'

	interface Props {
		id: string
	}

	let { id }: Props = $props()
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
