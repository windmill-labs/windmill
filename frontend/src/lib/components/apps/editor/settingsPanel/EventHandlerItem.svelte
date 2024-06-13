<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	import MultiSelect from '$lib/components/multiselect/MultiSelectWrapper.svelte'

	export let items: string[]
	export let value: string[] | undefined = undefined
	export let title: string
	export let tooltip: string
</script>

<div class="grid grid-cols-12 w-full gap-8">
	<div class="col-span-3 flex items-center text-xs whitespace-nowrap h-8">
		{title}
		<Tooltip light small>{tooltip}</Tooltip>
	</div>

	<div class="col-span-1" />

	<div class="col-span-8">
		{#if items.length === 0}
			<div
				class="text-xs text-secondary w-full bg-gray-100 border rounded-md flex items-center px-2 h-8 dark:bg-gray-900 dark:border-gray-600"
			>
				No components to recompute.
				<Tooltip light small>Create one and select it here.</Tooltip>
			</div>
		{:else}
			<MultiSelect {items} bind:value />
		{/if}
	</div>
</div>
