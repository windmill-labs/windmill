<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'

	import MultiSelect from '$lib/components/multiselect/MultiSelectWrapper.svelte'

	export let items: string[]
	export let value: string[] | undefined = undefined
	export let title: string
	export let tooltip: string
</script>

<div class="col-span-3 flex items-center text-xs whitespace-nowrap">
	{title}
	<Tooltip light small>{tooltip}</Tooltip>
</div>

<div class="col-span-7">
	{#if items.length === 0}
		<div
			class="text-xs text-secondary w-full bg-gray-100 border rounded-md h-7 flex items-center px-2"
		>
			No components to recompute.
			<Tooltip light small>Create one and select it here.</Tooltip>
		</div>
	{:else}
		<MultiSelect {items} bind:value />
	{/if}
</div>
