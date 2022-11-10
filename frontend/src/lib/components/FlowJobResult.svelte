<script lang="ts">
	import type { CompletedJob } from '$lib/gen'

	import DisplayResult from './DisplayResult.svelte'
	import LogViewer from './LogViewer.svelte'

	export let result: any
	export let logs: string
</script>

<div class="grid grid-cols-2 shadow border border-gray-800 h-full">
	<div class="bg-white max-h-80 h-full p-1 overflow-auto relative">
		<span class="text-gray-500">Result</span>
		<DisplayResult {result} />
	</div>
	<div class="overflow-auto max-h-80 h-full relative">
		<LogViewer content={logs ?? ''} isLoading={false} />
	</div>
</div>
