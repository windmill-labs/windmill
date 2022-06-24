<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let primary: boolean = true
</script>

<div class="flex flex-col sm:flex-row justify-between mt-4 mb-2">
	{#if primary}
		<h1>
			{title}
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</h1>
	{:else}
		<h2>
			{title}
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
		</h2>
	{/if}

	{#if $$slots.default}
		<div class="my-2 sm:ml-4">
			<slot />
		</div>
	{/if}
</div>
