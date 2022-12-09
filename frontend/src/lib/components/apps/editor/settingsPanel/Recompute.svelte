<script lang="ts">
	import PanelSection from './common/PanelSection.svelte'

	export let recomputeIds: string[] | undefined = undefined
</script>

<PanelSection title="Recompute">
	{#if recomputeIds}
		{#each recomputeIds as id}
			id
		{/each}
	{:else}
		No recompute
	{/if}
</PanelSection>
