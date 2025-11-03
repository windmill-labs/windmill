<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	interface Props {
		horizontal?: boolean
		gap?: 'none' | 'sm' | 'md' | 'lg'
		justify?: 'start' | 'center' | 'end' | 'between'
		wFull?: boolean
		hFull?: boolean
		children?: import('svelte').Snippet
	}

	let {
		horizontal = false,
		gap = 'sm',
		justify = 'start',
		wFull = true,
		hFull = true,
		children
	}: Props = $props()

	const gapMap = {
		none: '',
		sm: 'gap-2',
		md: 'gap-4',
		lg: 'gap-8'
	}

	const justifyMap = {
		start: 'justify-start',
		center: 'justify-center',
		end: 'justify-end',
		between: 'justify-between'
	}
</script>

{#if horizontal}
	<div
		class="flex flex-row h-full {wFull ? 'w-full' : ''} {gapMap[gap]} items-center {justifyMap[
			justify
		]}"
	>
		{@render children?.()}
	</div>
{:else}
	<div
		class={twMerge(
			'flex flex-col w-full',
			hFull ? 'h-full' : '',
			gapMap[gap],
			'items-center',
			justifyMap[justify]
		)}
	>
		{@render children?.()}
	</div>
{/if}
