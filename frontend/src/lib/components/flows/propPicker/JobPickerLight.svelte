<script lang="ts">
	import Cell from '$lib/components/table/Cell.svelte'
	import { formatDateShort, formatDate } from '$lib/utils'
	import type { Job } from '$lib/gen'
	import OutputBadge from './OutputBadge.svelte'

	export let job: Job | undefined
</script>

{#if job}
	<Cell class="w-full px-2">
		<OutputBadge class="w-full" {job} />
	</Cell>
	<Cell
		wrap
		class="text-2xs font-normal text-center p-2 overflow-auto text-ellipsis scrollbar-none"
		title={formatDate(job.created_at)}
	>
		{formatDateShort(job.created_at)}
	</Cell>
{/if}
