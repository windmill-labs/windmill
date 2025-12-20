<script lang="ts">
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import { ChevronDown } from 'lucide-svelte'

	interface Props {
		open?: boolean
		text: string
		class?: string
		children?: import('svelte').Snippet
	}

	let { open = $bindable(false), text, class: clazz = undefined, children }: Props = $props()
</script>

<div class="flex flex-col gap-1">
	<button
		class={twMerge('font-medium text-xs text-accent items-center mb-1 flex gap-1', clazz)}
		onclick={() => (open = !open)}
		type="button"
	>
		{text}
		<ChevronDown
			class={twMerge('transition-transform', open ? 'transform rotate-180' : '')}
			size={12}
		/>
	</button>

	{#if open}
		<div transition:slide|local={{ duration: 100 }}>{@render children?.()}</div>
	{/if}
</div>
