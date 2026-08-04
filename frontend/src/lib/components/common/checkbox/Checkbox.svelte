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
		/** Click handler, for the cases that need the modifier keys a `change`
		 * event doesn't carry (e.g. shift-click to select a range). Fires before
		 * `change`, so calling `preventDefault` here suppresses both. */
		onClick?: (e: MouseEvent & { currentTarget: EventTarget & HTMLInputElement }) => void
	}

	let {
		checked = false,
		indeterminate = false,
		disabled = false,
		title = undefined,
		class: className = undefined,
		onChange,
		onClick
	}: Props = $props()

	let inputEl: HTMLInputElement | undefined = $state()
	let clicks = $state(0)
	// A click flips the box before any handler runs, and flips it back when the
	// handler prevents the default — both outside Svelte's diff. So a controlled
	// checkbox whose value did not change across a click keeps the browser's
	// guess. Re-assert the controlled value after every click, not only when it
	// differs from the last rendered one.
	$effect(() => {
		clicks
		if (inputEl) inputEl.checked = checked
	})
</script>

<input
	bind:this={inputEl}
	type="checkbox"
	{checked}
	{indeterminate}
	{disabled}
	{title}
	onchange={onChange}
	onclick={(e) => {
		onClick?.(e)
		clicks++
	}}
	class={twMerge(
		'rounded max-w-4 w-full',
		// When disabled, grey it and let hover fall through to a wrapping trigger
		// (e.g. a tooltip explaining why it can't be selected).
		disabled ? 'opacity-50 cursor-not-allowed pointer-events-none' : '',
		className
	)}
/>
