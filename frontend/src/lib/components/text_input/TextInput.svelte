<script module lang="ts">
	export function inputBorderClass({
		error,
		forceFocus
	}: {
		error?: boolean
		forceFocus?: boolean
	} = {}) {
		return twMerge(
			'transition-colors border border-border-light',
			forceFocus
				? '!border-border-selected'
				: '!border-border-light hover:!border-border-selected/50 focus:!border-border-selected',
			error
				? '!border-red-300 dark:!border-red-400/45 focus:!border-red-400 hover:!border-red-500 dark:hover:!border-red-400/75'
				: ''
		)
	}

	export const inputBaseClass =
		'rounded-md focus:ring-0 no-default-style text-xs text-primary font-normal !bg-surface-input disabled:!bg-surface-disabled disabled:!border-transparent disabled:!text-disabled disabled:cursor-not-allowed shadow-none !placeholder-hint'

	import autosize from '$lib/autosize'
	import { ButtonType } from '$lib/components/common/button/model'

	export const inputSizeClasses = {
		xs: twMerge(
			ButtonType.UnifiedSizingClasses.xs,
			ButtonType.UnifiedMinHeightClasses.xs,
			'px-1 !py-0.5'
		),
		sm: twMerge(
			ButtonType.UnifiedSizingClasses.sm,
			ButtonType.UnifiedMinHeightClasses.sm,
			'px-2 !py-0.5'
		),
		md: twMerge(ButtonType.UnifiedSizingClasses.md, ButtonType.UnifiedMinHeightClasses.md, 'px-2'),
		lg: twMerge(ButtonType.UnifiedSizingClasses.lg, ButtonType.UnifiedMinHeightClasses.lg, 'px-2')
	}
</script>

<script lang="ts" generics="UnderlyingInputElT extends 'input' | 'textarea' = 'input'">
	import type { HTMLInputAttributes, HTMLTextareaAttributes } from 'svelte/elements'
	import { twMerge } from 'tailwind-merge'

	type Props<UnderlyingInputElT extends 'input' | 'textarea'> = {
		inputProps?: UnderlyingInputElT extends 'input' ? HTMLInputAttributes : HTMLTextareaAttributes
		value?: string | number
		class?: string
		error?: string | boolean
		size?: ButtonType.UnifiedSize
		unifiedHeight?: boolean
		underlyingInputEl?: UnderlyingInputElT
	}

	export function focus() {
		inputEl?.focus()
	}

	let inputEl: HTMLInputElement | HTMLTextAreaElement | undefined = $state()

	let {
		inputProps: _inputProps,
		value = $bindable(),
		class: className = '',
		error,
		size = 'md',
		unifiedHeight = true,
		underlyingInputEl: _underlyingInputEl
	}: Props<UnderlyingInputElT> = $props()

	let underlyingInputEl = $derived(_underlyingInputEl ?? ('input' as const))
	let inputProps = $derived(_inputProps as any)

	let fullClassName = $derived(
		twMerge(
			inputBaseClass,
			inputSizeClasses[size],
			'[appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none',
			inputBorderClass({ error: !!error }),
			unifiedHeight ? ButtonType.UnifiedHeightClasses[size] : '',
			'w-full',
			className
		)
	)
</script>

{#if underlyingInputEl === 'textarea'}
	<!-- We hardcode the py value because since textareas can be multiline, we can't rely on
 	  	 the fact that it auto-centers -->
	<textarea
		class={twMerge('py-[0.45rem]', fullClassName)}
		{...inputProps}
		onpointerdown={(e) => e.stopImmediatePropagation()}
		bind:this={inputEl}
		bind:value
		use:autosize
	></textarea>
{:else if underlyingInputEl === 'input'}
	<input
		{...inputProps}
		class={fullClassName}
		onpointerdown={(e) => e.stopImmediatePropagation()}
		bind:this={inputEl}
		bind:value
	/>
{/if}
