<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { ExternalLink } from 'lucide-svelte'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	export let documentationLink: string | undefined = undefined
	export let markdownTooltip: string | undefined = undefined
	const plugins = [gfmPlugin()]
</script>

<div
	class="shadow max-w-sm break-words py-2 px-3 rounded-md text-sm font-normal !text-gray-300 bg-gray-800 whitespace-normal text-left"
>
	{#if markdownTooltip}
		<div class="prose-sm">
			<Markdown md={markdownTooltip} {plugins} />
		</div>
	{:else}
		<slot />
	{/if}

	{#if documentationLink}
		<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
			<div class="flex flex-row gap-2 mt-4">
				See documentation
				<ExternalLink size="16" />
			</div>
		</a>
	{/if}
</div>
