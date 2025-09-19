<script module lang="ts">
	export function inputBorderClass({
		error,
		forceFocus
	}: {
		error?: boolean
		forceFocus?: boolean
	} = {}) {
		return twMerge(
			'transition-colors border',
			forceFocus
				? '!border-nord-900 dark:!border-nord-900'
				: '!border-transparent focus:!border-nord-900 dark:focus:!border-nord-900 hover:!border-nord-400 dark:hover:!border-nord-300',
			error
				? '!border-red-300 focus:!border-red-400 hover:!border-red-500 dark:!border-red-400/40 dark:hover:!border-red-600/40'
				: ''
		)
	}
</script>

<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements'
	import { twMerge } from 'tailwind-merge'

	type Props = {
		inputProps?: HTMLInputAttributes
		value?: string
		class?: string
		error?: string
	}

	let { inputProps, value = $bindable(), class: className = '', error }: Props = $props()
</script>

<input
	{...inputProps}
	class={twMerge(
		'no-default-style text-sm !bg-surface-secondary shadow-none py-2 px-4 w-full',
		'focus:ring-0',
		'[appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none',
		'rounded-md',
		inputBorderClass({ error: !!error }),
		className
	)}
	bind:value
/>
