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
	<MultiSelect {items} bind:value />
</div>
