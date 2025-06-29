<script lang="ts">
	import { Button } from '$lib/components/common'
	import { RotateCcw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	interface Props {
		id?: string | undefined
	}

	let { id = undefined }: Props = $props()

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')

	function onRefresh() {
		if (!id) return
		$componentControl[id]?.recompute?.()
	}
</script>

{#if id}
	<Button size="xs2" color="light" startIcon={{ icon: RotateCcw }} on:click={() => onRefresh()}>
		Force refresh
	</Button>
{/if}
