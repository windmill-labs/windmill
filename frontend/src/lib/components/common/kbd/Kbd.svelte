<script lang="ts">
	import { isMac } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		kbdClass?: string
		small?: boolean
		isModifier?: boolean
		classNames?: string
		children?: import('svelte').Snippet
	}

	let {
		kbdClass = $bindable(''),
		small = false,
		isModifier = false,
		classNames = '',
		children
	}: Props = $props()

	if (small) {
		kbdClass = twMerge(
			kbdClass,
			'!text-[10px]  px-1',
			isModifier && isMac() ? '!text-lg ' : 'text-xs',
			'leading-none'
		)
	} else {
		kbdClass += ' !text-xs px-1.5'
	}
</script>

<span
	class={twMerge(
		classNames,
		small ? 'h-4  center-center' : '',
		'ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse'
	)}
>
	<kbd class={kbdClass}>
		{@render children?.()}
	</kbd>
</span>
