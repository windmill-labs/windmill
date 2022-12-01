<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let primary: boolean = true
</script>

<div class="flex flex-row flex-wrap justify-between mt-4 mb-2 min-h-[48px]">
	{#if primary}
		<span class="flex items-center space-x-2">
			<h1 class="font-normal">{title}</h1>
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</span>
	{:else}
		<span class="flex items-center space-x-2">
			<h2>{title}</h2>
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
