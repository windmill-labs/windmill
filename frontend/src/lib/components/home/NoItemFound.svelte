<script lang="ts">
	interface Props {
		hasFilters?: boolean
	}

	let { hasFilters = false }: Props = $props()
</script>

{#if hasFilters}
	<div class="flex justify-center items-center h-48">
		<div class="text-primary text-center">
			<div class="text-lg font-semibold text-emphasis">No items found</div>
			<div class="text-xs font-normal text-hint">Try changing your search or filters</div>
		</div>
	</div>
{:else}
	<div class="flex justify-center items-center h-48">
		<div class="text-primary text-center">
			<div class="text-lg font-semibold text-emphasis">Welcome to Windmill</div>
			<div class="text-xs font-normal text-hint">Get started by creating your first script, flow, or app</div>
		</div>
	</div>
{/if}

