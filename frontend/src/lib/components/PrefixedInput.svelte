<script lang="ts">
	import { tick } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	let {
		prefix = '',
		value = $bindable(''),
		placeholder = '',
		class: className = '',
		autofocus = false,
		...restProps
	} = $props()

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
</script>

<!-- The prefix renders outside the <input> (faded, non-editable) while the wrapper
     carries the input look, so the two read as a single field. The bound value is
     only the editable part after the prefix. -->
<div
	class={twMerge(
		'flex items-center w-full min-h-8 px-2 rounded-md text-xs font-normal cursor-text',
		'bg-surface-input transition-colors border border-border-light',
		'hover:border-border-selected/50 focus-within:border-border-selected',
		className
	)}
>
	<!-- text-tertiary: lighter than typed text (it's not editable) but darker than the
	     hint-colored placeholder, so the fixed prefix doesn't read as placeholder. -->
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
