<script lang="ts">
	import { Building } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Tooltip } from './meltComponents'

	interface Props {
		class?: string
		children?: import('svelte').Snippet
	}

	let { class: className = '', children = undefined }: Props = $props()
</script>

<Tooltip>
	<div
		class={twMerge(
			'flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap px-1',
			className
		)}
		title="Enterprise Edition only feature"
		aria-label="Enterprise Edition only feature"
		role="tooltip"
	>
		EE only <Building size={16} />
	</div>
	{#snippet text()}
		{#if children}
			{@render children()}
		{:else}
			Enterprise Edition only feature
		{/if}
	{/snippet}
</Tooltip>
