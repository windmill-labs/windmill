<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		link?: string | undefined
		class?: string
		children?: import('svelte').Snippet
	}

	let { link = undefined, class: c = '', children }: Props = $props()
</script>

<div class={twMerge('text-xs text-primary font-normal', c)}>
	{@render children?.()}
	{#if link}
		<a href={link} target="_blank" class="whitespace-nowrap"
			>Learn more <ExternalLink size={12} class="inline-block" /></a
		>
	{/if}
</div>
