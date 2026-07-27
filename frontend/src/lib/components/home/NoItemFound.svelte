<script lang="ts">
	interface Props {
		hasFilters?: boolean
		// When provided (home page), it's the authoritative list of active filters:
		// non-empty means the current filters are too narrow (and are listed to the
		// user); empty means the workspace itself has no items. Takes precedence over
		// `hasFilters`, which other callers still use as a plain boolean.
		activeFilters?: string[]
	}

	let { hasFilters = false, activeFilters }: Props = $props()

	let narrowed = $derived(activeFilters != undefined ? activeFilters.length > 0 : hasFilters)
</script>

{#if narrowed}
	<div class="flex justify-center items-center h-48">
		<div class="text-primary text-center max-w-md px-4">
			<div class="text-lg font-semibold text-emphasis">No items match the current filters</div>
			{#if activeFilters && activeFilters.length > 0}
				<div class="text-xs font-normal text-hint mt-1">
					Active: {activeFilters.join(' · ')}
				</div>
				<div class="text-xs font-normal text-hint mt-0.5">Clear or widen them to see more.</div>
			{:else}
				<div class="text-xs font-normal text-hint">Try changing your search or filters</div>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex justify-center items-center h-48">
		<div class="text-primary text-center">
			<div class="text-lg font-semibold text-emphasis">Welcome to Windmill</div>
			<div class="text-xs font-normal text-hint">
				Get started by creating your first script, flow, or app
			</div>
		</div>
	</div>
{/if}
