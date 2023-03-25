<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let componentId: string
	export let loading: boolean
	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	async function refresh() {
		await $runnableComponents[componentId]?.cb?.()
	}
</script>

<button
	on:pointerdown|preventDefault|stopPropagation
	on:click|preventDefault|stopPropagation={refresh}
	class="center-center p-1 rounded border bg-white/60  hover:bg-gray-200 z-10"
>
	<RefreshCw class={loading ? 'animate-spin' : ''} size={16} />
</button>
