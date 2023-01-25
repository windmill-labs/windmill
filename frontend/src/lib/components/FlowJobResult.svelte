<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'

	export let result: any
	export let logs: string
	export let col: boolean = false
	export let noBorder = false
	export let loading
</script>

<div
	class:border={!noBorder}
	class="grid {!col ? 'grid-cols-2' : 'grid-rows-2'} shadow  border-gray-400 h-full"
>
	<div class="bg-white max-h-80 h-full p-1 overflow-auto relative">
		<span class="text-gray-500">Result</span>
		{#if result}
			<DisplayResult {result} />
		{:else if loading}
			<Loader2 class="animate-spin" />
		{:else}
			<div class="text-gray-400">No result (result is undefined)</div>
		{/if}
	</div>
	<div class="overflow-auto max-h-80 h-full relative">
		<LogViewer content={logs ?? ''} isLoading={false} />
	</div>
</div>
