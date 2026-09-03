<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	const dispatch = createEventDispatcher()

	interface Props {
		hoverable?: boolean;
		selected?: boolean;
		dividable?: boolean;
		disabled?: boolean;
		hovering?: boolean;
		class?: string;
		children?: import('svelte').Snippet;
	}

	let {
		hoverable = false,
		selected = false,
		dividable = false,
		disabled = false,
		hovering = $bindable(false),
		class: className = '',
		children
	}: Props = $props();
	
</script>

<tr
	class={twMerge(
		// `wm-row-hoverable` lets a pinned cell re-create this tint on top of its own opaque
		// background — it cannot simply adopt it, since the hover token carries alpha.
		hoverable ? 'wm-row-hoverable hover:bg-surface-hover cursor-pointer' : '',
		selected ? 'bg-blue-50 dark:bg-blue-900/50' : '',
		'transition-all',
		dividable ? 'divide-x' : '',
		disabled ? 'opacity-60' : '',
		className
	)}
	onclick={() => {
		dispatch('click')
	}}
	onmouseenter={() => {
		hovering = true
		dispatch('hover', true)
	}}
	onmouseleave={() => {
		hovering = false
		dispatch('hover', false)
	}}
>
	{@render children?.()}
</tr>
