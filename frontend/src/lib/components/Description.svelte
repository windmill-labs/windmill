<script lang="ts">
	import { ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let link: string | undefined = undefined
</script>

<div class={twMerge('text-xs text-primary font-normal', $$props.class)}>
	<slot />
	{#if link}
		<a href={link} target="_blank" class="whitespace-nowrap"
			>Learn more <ExternalLink size={12} class="inline-block" /></a
		>
	{/if}
</div>
