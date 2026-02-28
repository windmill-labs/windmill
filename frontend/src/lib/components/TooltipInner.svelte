<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let documentationLink: string | undefined = undefined
	export let markdownTooltip: string | undefined = undefined
	export let customBgClass: string | undefined = undefined

	let markdownModule: Promise<{ default: typeof import('svelte-exmarkdown').default; gfmPlugin: typeof import('svelte-exmarkdown/gfm').gfmPlugin }> | undefined = undefined
	$: if (markdownTooltip && !markdownModule) {
		markdownModule = Promise.all([
			import('svelte-exmarkdown'),
			import('svelte-exmarkdown/gfm')
		]).then(([md, gfm]) => ({ default: md.default, gfmPlugin: gfm.gfmPlugin }))
	}
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
		<slot />
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
