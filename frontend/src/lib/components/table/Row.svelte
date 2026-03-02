<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		hoverable?: boolean
		selected?: boolean
		dividable?: boolean
		disabled?: boolean
		hovering?: boolean
		class?: string
		onclick?: () => void
		onHover?: (hovering: boolean) => void
		children?: import('svelte').Snippet
	}

	let {
		hoverable = false,
		selected = false,
		dividable = false,
		disabled = false,
		hovering = $bindable(false),
		class: c = '',
		onclick,
		onHover,
		children
	}: Props = $props()
</script>

<tr
	class={twMerge(
		hoverable ? 'hover:bg-surface-hover cursor-pointer' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all',
		dividable ? 'divide-x' : '',
		disabled ? 'opacity-60' : '',
		c
	)}
	onclick={() => {
		onclick?.()
	}}
	onmouseenter={() => {
		hovering = true
		onHover?.(true)
	}}
	onmouseleave={() => {
		hovering = false
		onHover?.(false)
	}}
>
	{@render children?.()}
</tr>
