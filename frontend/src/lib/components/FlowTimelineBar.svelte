<script lang="ts">
	import TimelineBar from './TimelineBar.svelte'

	export let total: number
	export let min: number | number
	export let now: number
	export let duration_ms: number | undefined
	export let started_at: number | undefined
	export let id: string
</script>

{#if started_at}
	<!-- {started_at}
	{duration_ms} -->
	<TimelineBar
		{id}
		{total}
		{min}
		{started_at}
		len={duration_ms ?? now - started_at}
		running={duration_ms == undefined}
	/>
{/if}
