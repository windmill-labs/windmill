<script lang="ts">
	import { msToSec } from '$lib/utils'

	interface Props {
		total: number
		min: number | undefined
		started_at: number | undefined
		duration_ms: number | undefined
		running: boolean
	}

	let { total, min, started_at, duration_ms, running }: Props = $props()

	let len = $derived(duration_ms ?? 0)
</script>

{#if min && started_at != undefined}
	<div class="flex items-center gap-2">
		<div class="flex-1 h-1 bg-gray-50 dark:bg-gray-800 rounded-sm overflow-hidden">
			<div style="width: {((started_at - min) / total) * 100}%" class="h-full float-left"></div>
			<div
				style="width: {(len / total) * 100}%"
				class="h-full float-left {running ? 'bg-blue-400' : 'bg-blue-500'}"
			></div>
		</div>
		{#if len > 0}
			<span class="text-xs text-tertiary font-mono">{msToSec(len, 1)}s</span>
		{/if}
	</div>
{/if}
