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
		'2xs': twMerge(
			ButtonType.UnifiedSizingClasses['2xs'],
			ButtonType.UnifiedMinHeightClasses['2xs'],
			'px-1 !py-0.5'
		),
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

	// Base leading == (unified height − vertical padding) so a single-line
	// contenteditable div centers its text the way a native <input> does.
	//
	// In "large mode" (viewport ≥ 1760px, where app.css bumps :root to 18px →
	// font 13.5px) headless-Chromium ink measurement showed the text sitting
	// ~1px low with the base leading. The residual is a fixed ~2px of line box,
	// so the exact centered value there is (content-box height − 2px). Scoped to
	// the same 1760px breakpoint as the font-size bump; small mode is unchanged.
	export const inputLeadingClasses: Record<ButtonType.UnifiedSize, string> = {
		'2xs': 'leading-4 min-[1760px]:leading-[calc(1rem_-_2px)]', // h-5 − py-0.5 → 1rem
		xs: 'leading-4 min-[1760px]:leading-[calc(1rem_-_2px)]', // h-5 − py-0.5 → 1rem
		sm: 'leading-6 min-[1760px]:leading-[calc(1.5rem_-_2px)]', // h-7 − py-0.5 → 1.5rem
		md: 'leading-8 min-[1760px]:leading-[calc(2rem_-_2px)]', // h-8, no py → 2rem
		lg: 'leading-10 min-[1760px]:leading-[calc(2.5rem_-_2px)]' // h-10, no py → 2.5rem
	}
</script>

<script lang="ts" generics="UnderlyingInputElT extends 'input' | 'textarea' | 'div' = 'input'">
	import type { HTMLAttributes, HTMLInputAttributes, HTMLTextareaAttributes } from 'svelte/elements'
	import { twMerge } from 'tailwind-merge'

	type DivInputProps = HTMLAttributes<HTMLDivElement> & {
		disabled?: boolean
		placeholder?: string
	}

	type Props<UnderlyingInputElT extends 'input' | 'textarea' | 'div'> = {
		inputProps?: UnderlyingInputElT extends 'input'
			? HTMLInputAttributes
			: UnderlyingInputElT extends 'textarea'
				? HTMLTextareaAttributes
				: DivInputProps
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

	export function select() {
		if (inputEl instanceof HTMLDivElement) {
			const range = document.createRange()
			range.selectNodeContents(inputEl)
			const sel = window.getSelection()
			sel?.removeAllRanges()
			sel?.addRange(range)
		} else {
			inputEl?.select()
		}
	}

	let inputEl: HTMLInputElement | HTMLTextAreaElement | HTMLDivElement | undefined = $state()

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
	let isDiv = $derived(underlyingInputEl === 'div')
	let divDisabled = $derived(isDiv && Boolean(inputProps?.disabled))

	let fullClassName = $derived(
		twMerge(
			inputBaseClass,
			inputSizeClasses[size],
			'[appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none',
			inputBorderClass({ error: !!error }),
			unifiedHeight ? ButtonType.UnifiedHeightClasses[size] : '',
			'w-full',
			isDiv &&
				`whitespace-pre overflow-hidden ${inputLeadingClasses[size]} focus:outline-none focus:ring-0 focus-visible:outline-none focus-visible:ring-0 empty:before:content-[attr(data-placeholder)] empty:before:text-hint`,
			divDisabled && '!bg-surface-disabled !border-transparent !text-disabled cursor-not-allowed',
			className
		)
	)

	// contenteditable is mutated by the browser as the user types — a templated
	// `>{value}</div>` would race with that and produce duplicated text. Sync
	// imperatively, with an equality guard so user input doesn't echo back.
	$effect(() => {
		if (underlyingInputEl !== 'div' || !inputEl) return
		const target = value == null ? '' : String(value)
		if (inputEl.textContent !== target) {
			inputEl.textContent = target
		}
	})
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
{:else if underlyingInputEl === 'div'}
	{@const { disabled, placeholder, ...divProps } = (inputProps ?? {}) as DivInputProps}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		role="textbox"
		aria-disabled={disabled}
		tabindex={disabled ? -1 : 0}
		contenteditable={!disabled}
		{...divProps}
		onkeydown={(e) => {
			if (e.key === 'Enter') e.preventDefault()
			divProps?.onkeydown?.(e)
		}}
		class={fullClassName}
		data-placeholder={placeholder ?? ''}
		onpointerdown={(e) => e.stopImmediatePropagation()}
		oninput={(e) => {
			value = e.currentTarget.textContent ?? ''
		}}
		bind:this={inputEl}
	></div>
{/if}
