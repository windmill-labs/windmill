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
				: '!border-nord-400 dark:!border-nord-300 hover:!border-nord-900/50 hover:dark:!border-nord-900/50 focus:!border-nord-900 dark:focus:!border-nord-900',
			error
				? '!border-red-300 focus:!border-red-400 hover:!border-red-500 dark:!border-red-400/40 dark:hover:!border-red-600/40'
				: ''
		)
	}

	export const inputBaseClass =
		'rounded-md focus:ring-0 no-default-style text-sm text-tertiary !bg-surface-secondary disabled:!bg-surface-disabled disabled:!text-hint disabled:cursor-not-allowed shadow-none py-2 px-4 placeholder-hint'
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

	export function focus() {
		inputEl?.focus()
	}

	let inputEl: HTMLInputElement | undefined = $state()

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
	bind:this={inputEl}
	bind:value
/>
