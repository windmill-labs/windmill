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

	export const inputBaseClass =
		'rounded-md focus:ring-0 no-default-style text-sm !bg-surface-secondary disabled:!bg-surface-disabled disabled:cursor-not-allowed shadow-none py-2 px-4'
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
		inputBaseClass,
		'w-full',
		'[appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none',
		inputBorderClass({ error: !!error }),
		className
	)}
	bind:value
/>
