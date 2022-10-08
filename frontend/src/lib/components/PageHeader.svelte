<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let primary: boolean = true
</script>

<div class="flex flex-col sm:flex-row justify-between mt-4 mb-2">
	{#if primary}
		<span class="text-xl font-bold flex items-center space-x-2">
			<span class="mr-0.5">{title}</span>
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</span>
		{:else}
		<span class="text-lg font-semibold flex items-center space-x-2">
			<span class="mr-0.5">{title}</span>
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</span>
	{/if}

	{#if $$slots.default}
		<div class="my-2">
			<slot />
		</div>
	{/if}
</div>
