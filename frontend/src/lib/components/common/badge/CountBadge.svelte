<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		count: number | undefined
		small?: boolean
		alwaysVisible?: boolean
	}

	let { count, small = true, alwaysVisible = false }: Props = $props()
</script>

{#if count === undefined || count > 0 || alwaysVisible}
	<div
		class={twMerge(
			// Base styles that apply in all cases
			'absolute z-10 rounded-full overflow-hidden',
			'flex center-center text-primary-inverse font-mono',
			'bg-tertiary/50 group-hover:bg-primary transition-all duration-[100ms]',
			alwaysVisible || count === undefined ? 'bg-primary' : '',

			// Size variants based on small prop
			small ? '-right-[3px] -top-[3px] h-3 w-3 text-[8px]' : '-right-1.5 -top-1.5 h-4 w-4 text-xs',

			// Special case for always visible
			alwaysVisible && small ? 'h-3 w-3 text-[8px] -right-0.5 -top-0.5' : '',
			alwaysVisible && !small ? 'h-4 w-4 text-xs -right-1 -top-1' : ''
		)}
	>
		{#if count === undefined}
			<Loader2 class="animate-spin text-2xs" />
		{:else}
			<p>{count}</p>
		{/if}
	</div>
{/if}