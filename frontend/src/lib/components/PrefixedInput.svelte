<script lang="ts">
	import { tick } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { HTMLInputAttributes } from 'svelte/elements'

	let {
		prefix = '',
		value = $bindable(),
		placeholder = '',
		error = false,
		autofocus = false,
		class: className = '',
		...restProps
	}: {
		prefix?: string
		value: string
		placeholder?: string
		// Red border, matching TextInput's error styling.
		error?: boolean
		autofocus?: boolean
		class?: string
	} & Omit<
		HTMLInputAttributes,
		'prefix' | 'value' | 'placeholder' | 'autofocus' | 'class'
	> = $props()

	let inputElement: HTMLInputElement | undefined = $state()

	// Programmatic focus: the native autofocus attribute is unreliable on content
	// mounted after page load (e.g. inside a modal).
	$effect(() => {
		if (!autofocus) return
		void tick().then(() => {
			inputElement?.focus()
			inputElement?.select()
		})
	})

	const borderClasses = $derived(
		error
			? 'border !border-red-300 dark:!border-red-400/45 focus-within:!border-red-400 hover:!border-red-500 dark:hover:!border-red-400/75'
			: 'border border-border-light hover:border-border-selected/50 focus-within:border-border-selected'
	)
</script>

<!-- The prefix renders outside the <input> (non-editable) while the wrapper
     carries the input look, so the two read as a single field. The bound value
     is only the editable part after the prefix. The prefix is text-tertiary:
     lighter than typed text but darker than the hint-colored placeholder, so
     the fixed prefix doesn't read as placeholder. -->
<div
	class={twMerge(
		'flex items-center w-full min-h-8 px-2 rounded-md text-xs font-normal cursor-text',
		'bg-surface-input transition-colors',
		borderClasses,
		className
	)}
>
	<span class="text-tertiary select-none shrink-0" aria-hidden="true">{prefix}</span>
	<input
		bind:this={inputElement}
		type="text"
		{placeholder}
		bind:value
		class="no-default-style grow min-w-0 !p-0 !border-none !outline-none focus:!ring-0 !bg-transparent text-xs text-primary !placeholder-hint"
		{...restProps}
	/>
</div>
