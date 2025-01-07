<script lang="ts">
	import CaptureTable from '$lib/components/triggers/CaptureTable.svelte'

	export let flowPath: string | null = null
</script>

<div class="h-full">
	<CaptureTable path={flowPath ?? ''} on:select isFlow={true} headless={true} />
</div>
