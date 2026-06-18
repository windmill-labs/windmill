<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	interface Props {
		title: string;
		tooltip?: string;
		documentationLink?: string | undefined;
		primary?: boolean;
		childrenWrapperDivClasses?: string;
		children?: import('svelte').Snippet;
	}

	let {
		title,
		tooltip = '',
		documentationLink = undefined,
		primary = true,
		childrenWrapperDivClasses = '',
		children
	}: Props = $props();
</script>

<div class="flex flex-row flex-wrap justify-between items-center pb-2 my-4 mr-2 min-h-16">
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

	{#if children}
		<div class="my-2 {childrenWrapperDivClasses}">
			{@render children?.()}
		</div>
	{/if}
</div>
