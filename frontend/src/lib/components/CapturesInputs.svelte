<script lang="ts">
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'

	export let flowPath: string | null = null
	export let headless: boolean = true
	export let addButton: boolean = false
</script>

<div class="h-full">
	<CaptureTable
		path={flowPath ?? ''}
		on:select
		isFlow={true}
		{headless}
		{addButton}
		on:openTriggers
	/>
</div>
