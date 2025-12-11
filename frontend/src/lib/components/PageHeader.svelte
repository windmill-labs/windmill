<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	export let title: string
	export let tooltip: string = ''
	export let documentationLink: string | undefined = undefined
	export let primary: boolean = true
	export let childrenWrapperDivClasses: string = ''
</script>

<div class="flex flex-row flex-wrap justify-between pb-2 my-4 mr-2">
	{#if primary}
		<span class="flex items-center gap-2">
			<h1 class="text-2xl font-semibold text-emphasis whitespace-nowrap leading-6 tracking-tight"
				>{title}</h1
			>
			{#if tooltip != '' || documentationLink}
				<Tooltip {documentationLink}>
					{tooltip}
				</Tooltip>
			{/if}
		</span>
	{:else}
		<span class="flex items-center gap-2">
			<h2 class="text-sm font-semibold text-emphasis">{title}</h2>
			{#if tooltip != '' || documentationLink}
				<Tooltip {documentationLink} wrapperClass="mb-0.5">
					{tooltip}
				</Tooltip>
			{/if}
		</span>
	{/if}

	{#if $$slots.default}
		<div class="my-2 {childrenWrapperDivClasses}">
			<slot />
		</div>
	{/if}
</div>
