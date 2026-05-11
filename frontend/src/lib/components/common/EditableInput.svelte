<!--
@component
Inline-editable text. Renders as a static button in idle mode; clicking it
swaps to a `TextInput` whose width tracks the content (no layout shift on
toggle). `Enter` or `blur` commits via `onSave`; `Escape` discards.

Use it for header titles, summaries, list-item names — anywhere the user
should be able to edit a label in place without opening a modal or popover.

```svelte
<EditableInput
  value={summary}
  placeholder="Add a summary..."
  onSave={(v) => (summary = v)}
  textClass="text-xs font-semibold text-emphasis"
/>
```

The current value isn't bound — `onSave` is fired with the trimmed draft
whenever it differs from the prior `value`, including with `''` when the
user clears the field. Callers that want to reject empty commits should
guard inside their `onSave` handler. The parent owns the canonical state;
this component just proposes new values.
-->
<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		/** Current value displayed in idle mode and pre-filled when entering edit mode. */
		value: string
		/** Shown when `value` is empty, in both idle and editing modes. */
		placeholder?: string
		/**
		 * Called when the user commits a changed value (Enter or blur). Fires
		 * with the trimmed draft, including `''` if the user cleared the field.
		 * Not called on Escape, or on blur when the trimmed draft matches the
		 * prior `value`. Guard against empty in your handler if needed.
		 */
		onSave?: (newValue: string) => void
		/** When false, the component renders as plain text (not clickable). Default true. */
		editable?: boolean
		/** TextInput size in editing mode. Idle mode is unaffected (text only). */
		size?: 'xs' | 'sm' | 'md' | 'lg'
		/** Wrapper classes. Use for layout (margin, max-width, alignment) only — not text styling. */
		class?: string
		/** Extra classes on the inner `<input>` in editing mode. Background/border/shadow are reset on top of these. */
		inputClass?: string
		/**
		 * Text styling (font-size, weight, color, line-height...) applied to *both*
		 * the idle button and the editing input so the two render identically and
		 * the toggle doesn't visually shift.
		 */
		textClass?: string
	}

	let {
		value,
		placeholder = '',
		onSave,
		editable = true,
		size = 'sm',
		class: className = '',
		inputClass = '',
		textClass = ''
	}: Props = $props()

	let editing = $state(false)
	let draft = $state('')
	let textInputComponent: TextInput | undefined = $state(undefined)

	function startEditing() {
		if (!editable) return
		editing = true
		draft = value ?? ''
		requestAnimationFrame(() => {
			textInputComponent?.focus()
			textInputComponent?.select()
		})
	}

	function save() {
		// Re-entry guard: Enter calls `save()` and sets `editing = false`,
		// which unmounts the `<input>` and synchronously fires its `blur`
		// handler — also `save()`. Without this guard, `onSave` would fire
		// twice for the same edit.
		if (!editing) return
		editing = false
		const trimmed = draft.trim()
		if (trimmed !== (value ?? '')) {
			onSave?.(trimmed)
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') save()
		else if (e.key === 'Escape') editing = false
	}
</script>

{#if editing}
	<span
		class="input-sizer inline-grid items-center {textClass} {className}"
		data-value={draft || placeholder}
	>
		<TextInput
			bind:this={textInputComponent}
			bind:value={draft}
			{size}
			class="!bg-transparent !border-0 !shadow-none -p-0 !m-0 !min-w-0 !min-h-0 !h-auto {textClass} {inputClass}"
			inputProps={{
				placeholder,
				onblur: save,
				onkeydown: handleKeydown,
				spellcheck: false,
				size: 1,
				style: 'padding: 2px !important; grid-area: 1 / 1'
			}}
		/>
	</span>
{:else}
	<button
		type="button"
		onclick={startEditing}
		disabled={!editable}
		aria-label={editable ? `Edit ${placeholder.toLowerCase() || 'value'}` : undefined}
		class="text-left truncate rounded p-0.5 {editable
			? 'cursor-text hover:bg-surface-hover'
			: 'cursor-default'} {textClass} {className}"
	>
		{value || placeholder}
	</button>
{/if}

<style>
	.input-sizer::after {
		content: attr(data-value) ' ';
		visibility: hidden;
		white-space: pre;
		grid-area: 1 / 1;
		font: inherit;
		padding: 2px;
	}
</style>
