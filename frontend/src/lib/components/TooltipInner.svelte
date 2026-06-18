<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		documentationLink?: string | undefined
		customBgClass?: string | undefined
		children?: import('svelte').Snippet
	}

	let {
		documentationLink = undefined,
		customBgClass = undefined,
		children
	}: Props = $props()
</script>

<div
	class={twMerge(
		'shadow-lg max-w-sm break-words py-2 px-3 rounded-md text-xs font-normal text-primary  whitespace-normal text-left dark:border max-h-64 overflow-y-auto',
		customBgClass || 'bg-surface-secondary'
	)}
>
	{@render children?.()}

	{#if documentationLink}
		<a href={documentationLink} target="_blank">
			<div class="flex flex-row gap-2 mt-4">
				See documentation
				<ExternalLink size="16" />
			</div>
		</a>
	{/if}
</div>
