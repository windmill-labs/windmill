<script lang="ts">
	import type { Snippet } from 'svelte'
	import type { File } from 'lucide-svelte'

	interface Props {
		icon: typeof File
		title: string
		/** Shown on hover, when the title is shortened. */
		titleTooltip?: string | undefined
		subtitle: Snippet
		subtitleTooltip?: string | undefined
		actions?: Snippet | undefined
	}

	let { icon: Icon, title, titleTooltip, subtitle, subtitleTooltip, actions }: Props = $props()
</script>

<!-- Wraps the actions onto their own line before the title gets too narrow to read. -->
<div class="flex flex-row flex-wrap items-center gap-x-3 gap-y-2 min-w-0">
	<Icon size={20} class="shrink-0 text-secondary" />
	<div class="flex flex-col min-w-20 flex-1">
		<span class="text-xs font-semibold text-emphasis truncate" title={titleTooltip ?? title}
			>{title}</span
		>
		<span class="text-2xs text-secondary truncate" title={subtitleTooltip}>
			{@render subtitle()}
		</span>
	</div>
	{#if actions}
		<div class="flex flex-row items-center gap-1 shrink-0 ml-auto">
			{@render actions()}
		</div>
	{/if}
</div>
