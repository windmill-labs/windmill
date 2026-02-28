<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		documentationLink?: string | undefined
		markdownTooltip?: string | undefined
		customBgClass?: string | undefined
		children?: import('svelte').Snippet
	}

	let {
		documentationLink = undefined,
		markdownTooltip = undefined,
		customBgClass = undefined,
		children
	}: Props = $props()

	let markdownModule: Promise<{ default: typeof import('svelte-exmarkdown').default; gfmPlugin: typeof import('svelte-exmarkdown/gfm').gfmPlugin }> | undefined = $state()
	$effect(() => {
		if (markdownTooltip && !markdownModule) {
			markdownModule = Promise.all([
				import('svelte-exmarkdown'),
				import('svelte-exmarkdown/gfm')
			]).then(([md, gfm]) => ({ default: md.default, gfmPlugin: gfm.gfmPlugin }))
		}
	})
</script>

<div
	class={twMerge(
		'shadow-lg max-w-sm break-words py-2 px-3 rounded-md text-xs font-normal text-primary  whitespace-normal text-left dark:border max-h-64 overflow-y-auto',
		customBgClass || 'bg-surface-secondary'
	)}
>
	{#if markdownTooltip && markdownModule}
		{#await markdownModule then { default: Markdown, gfmPlugin }}
			<div class="prose-sm">
				<Markdown md={markdownTooltip} plugins={[gfmPlugin()]} />
			</div>
		{/await}
	{:else}
		{@render children?.()}
	{/if}

	{#if documentationLink}
		<a href={documentationLink} target="_blank">
			<div class="flex flex-row gap-2 mt-4">
				See documentation
				<ExternalLink size="16" />
			</div>
		</a>
	{/if}
</div>
