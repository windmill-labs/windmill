<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		/** Controlled checked state. */
		checked?: boolean
		/** Tri-state display (e.g. a group header with only part of its items
		 * selected). Purely visual — `checked` still drives the value. */
		indeterminate?: boolean
		disabled?: boolean
		/** Native title attribute (hover hint). */
		title?: string | undefined
		/** Extra classes merged onto the input. */
		class?: string | undefined
		/** Change handler (controlled — the parent owns `checked`). */
		onChange?: (e: Event & { currentTarget: EventTarget & HTMLInputElement }) => void
	}

	let {
		checked = false,
		indeterminate = false,
		disabled = false,
		title = undefined,
		class: className = undefined,
		onChange
	}: Props = $props()
</script>

<input
	type="checkbox"
	{checked}
	{indeterminate}
	{disabled}
	{title}
	onchange={onChange}
	class={twMerge(
		'rounded max-w-4 w-full',
		// When disabled, grey it and let hover fall through to a wrapping trigger
		// (e.g. a tooltip explaining why it can't be selected).
		disabled ? 'opacity-50 cursor-not-allowed pointer-events-none' : '',
		className
	)}
/>
