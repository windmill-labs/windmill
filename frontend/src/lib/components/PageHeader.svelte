<script lang="ts">
	import Tooltip from './Tooltip.svelte'

	interface Props {
		title: string
		tooltip?: string
		documentationLink?: string | undefined
		primary?: boolean
		childrenWrapperDivClasses?: string
		// Inline actions rendered right after the title (e.g. a copy-id button),
		// as opposed to `children` which lands on the far right of the header row.
		titleActions?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		title,
		tooltip = '',
		documentationLink = undefined,
		primary = true,
		childrenWrapperDivClasses = '',
		titleActions,
		children
	}: Props = $props()
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
			{@render titleActions?.()}
		</span>
	{:else}
		<span class="flex items-center gap-2">
			<h2 class="text-sm font-semibold text-emphasis">{title}</h2>
			{#if tooltip != '' || documentationLink}
				<Tooltip {documentationLink} wrapperClass="mb-0.5">
					{tooltip}
				</Tooltip>
			{/if}
			{@render titleActions?.()}
		</span>
	{/if}

	{#if children}
		<div class="my-2 {childrenWrapperDivClasses}">
			{@render children?.()}
		</div>
	{/if}
</div>
