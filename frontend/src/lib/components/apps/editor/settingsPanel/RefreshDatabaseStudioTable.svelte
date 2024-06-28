<script lang="ts">
	import { Button } from '$lib/components/common'
	import { RotateCcw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let id: string | undefined = undefined

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')

	function onRefresh() {
		if (!id) return
		console.log('Refreshing table', $componentControl[id])
		$componentControl[id]?.recompute?.()
	}
</script>

{#if id}
	<Button size="xs2" color="light" startIcon={{ icon: RotateCcw }} on:click={() => onRefresh()}>
		Force refresh
	</Button>
{/if}
