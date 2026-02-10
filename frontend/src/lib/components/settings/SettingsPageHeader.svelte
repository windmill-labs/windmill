<script lang="ts">
	import Description from '$lib/components/Description.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		title: string
		description?: string
		link?: string
		actions?: import('svelte').Snippet
		class?: string
	}

	let { title, description, link, actions, class: className = '' }: Props = $props()
</script>

<div class={twMerge('flex flex-col gap-2 mb-6', className)}>
	<div class="flex items-center justify-between gap-4">
		<div class="flex-1 min-w-0">
			<h2 class="text-lg font-semibold text-emphasis">{title}</h2>
		</div>
		{#if actions}
			<div class="flex items-center gap-2 shrink-0">
				{@render actions()}
			</div>
		{/if}
	</div>
	{#if description}
		<Description {link}>
			{@html description}
		</Description>
	{/if}
</div>
