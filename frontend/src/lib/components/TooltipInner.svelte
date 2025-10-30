<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { ExternalLink } from 'lucide-svelte'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	interface Props {
		documentationLink?: string | undefined;
		markdownTooltip?: string | undefined;
		children?: import('svelte').Snippet;
	}

	let { documentationLink = undefined, markdownTooltip = undefined, children }: Props = $props();
	const plugins = [gfmPlugin()]
</script>

<div
	class="shadow-lg max-w-sm break-words py-2 px-3 rounded-md text-xs font-normal text-primary bg-surface-secondary whitespace-normal text-left dark:border"
>
	{#if markdownTooltip}
		<div class="prose-sm">
			<Markdown md={markdownTooltip} {plugins} />
		</div>
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
