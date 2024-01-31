<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let documentationLink: string | undefined = undefined
	export let primary: boolean = true
</script>

<div class="flex flex-row flex-wrap justify-between pt-6 pb-2 my-4">
	{#if primary}
		<span class="flex items-center gap-2">
			<h1 class="!text-2xl font-semibold leading-6 tracking-tight">{title}</h1>
			{#if tooltip != '' || documentationLink}
				<Tooltip {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</span>
	{:else}
		<span class="flex items-center gap-2">
			<h2 class="!text-sm font-semibold">{title}</h2>
			{#if tooltip != '' || documentationLink}
				<Tooltip {documentationLink} wrapperClass="mb-0.5">
					{tooltip}
				</Tooltip>
			{/if}
		</span>
	{/if}

	{#if $$slots.default}
		<div class="my-2">
			<slot />
		</div>
	{/if}
</div>
