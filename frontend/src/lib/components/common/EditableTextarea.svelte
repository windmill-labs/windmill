<!--
@component
Inline-editable prose. The multiline counterpart to `EditableInput`: idle renders
the whole text, wrapped, as a clickable button; clicking swaps to an auto-growing
`textarea`. `Enter` or `blur` commits via `onSave`, `Shift+Enter` adds a line,
`Escape` discards — except under `commitOnInput`, where every keystroke has
already been propagated and there is nothing left to discard.

Escape is stopped from propagating, which keeps a surrounding `Drawer` open. A `Modal`
is the exception: it takes both Enter and Escape on `window` in the capture phase and
stops them there, so neither reaches this. Inside one, pass `enterConfirms={false}` to
the `Modal` for Enter to commit here rather than confirm the dialog; Escape stays the
dialog's either way, so expect it to leave the edit and close the dialog together.

Use it where a cell or a field holds a sentence rather than a label — a prompt, a
description, an expected answer. For a one-line label, use `EditableInput`.

```svelte
<EditableTextarea
  value={question}
  placeholder="Question"
  onSave={(v) => (question = v)}
  textClass="text-xs font-normal"
/>
```

Like `EditableInput`, the value isn't bound: `onSave` fires whenever the draft
differs from the prior `value`, including `''` when cleared. Unlike it, the draft
goes back verbatim — whitespace a caller stored on purpose survives a focus and a
blur. The parent owns the canonical state.
-->
<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	interface Props {
		/** Current value shown in idle mode and pre-filled when entering edit mode. */
		value: string
		/** Shown when `value` is empty, in both idle and editing modes. */
		placeholder?: string
		/**
		 * Called when the user commits a changed value (Enter or blur, or every keystroke when
		 * {@link commitOnInput} is set). Fires with the draft exactly as typed, including `''` if the
		 * field was cleared. Not called on Escape, or when the draft matches the prior `value`.
		 */
		onSave?: (newValue: string) => void
		/**
		 * Fire `onSave` on every keystroke instead of only on Enter/blur. Use when the parent
		 * autosaves, or when a control outside this one reads the value on click — the click blurs
		 * the textarea, and a commit-on-blur would land after the read. Escape no longer discards
		 * with this on: the live commits have already propagated.
		 */
		commitOnInput?: boolean
		/** When false, renders as plain text and is not clickable. */
		editable?: boolean
		/** Textarea size in editing mode. Idle mode is text only and unaffected. */
		size?: 'xs' | 'sm' | 'md' | 'lg'
		/** Wrapper classes. Layout only — margin, width, alignment — not text styling. */
		class?: string
		/** Extra classes on the `<textarea>`. Background, border and shadow are reset over these. */
		inputClass?: string
		/**
		 * Text styling applied to *both* the idle text and the textarea, so the two render
		 * identically and clicking in does not shift the text.
		 */
		textClass?: string
	}

	let {
		value,
		placeholder = '',
		onSave,
		commitOnInput = false,
		editable = true,
		size = 'sm',
		class: className = '',
		inputClass = '',
		textClass = ''
	}: Props = $props()

	let editing = $state(false)
	let draft = $state('')
	// `TextInput` is generic over its underlying element and defaults to `'input'`; this one renders
	// a textarea, so the binding has to say so or it is typed as the wrong component.
	let textInputComponent: TextInput<'textarea'> | undefined = $state(undefined)

	function startEditing() {
		if (!editable) return
		editing = true
		draft = value ?? ''
		requestAnimationFrame(() => {
			textInputComponent?.focus()
			textInputComponent?.select()
		})
	}

	/** Open the editor from outside — a row that was just added, say, so the caller does not have
	 *  to click into it. */
	export function edit() {
		startEditing()
	}

	function save() {
		// Re-entry guard: Enter calls `save()` and clears `editing`, which unmounts the textarea and
		// synchronously fires its `blur` handler — also `save()`. Without this, `onSave` fires twice.
		if (!editing) return
		editing = false
		// Verbatim, not trimmed: a caller's value may hold whitespace that means something, and
		// merely focusing and blurring a field must never count as an edit.
		if (draft !== (value ?? '')) {
			onSave?.(draft)
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			// Kept from a surrounding drawer, which would otherwise close on it. A `Modal` takes
			// Escape at `window` in the capture phase and is past us before this runs.
			e.preventDefault()
			e.stopPropagation()
			editing = false
			return
		}
		// Shift+Enter falls through to the textarea's own newline; plain Enter commits, which is
		// what makes this usable in a table where Enter means "done with this cell".
		if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
			e.preventDefault()
			save()
		}
	}

	function handleLiveInput(e: Event) {
		const next = (e.currentTarget as HTMLTextAreaElement).value
		if (next !== (value ?? '')) onSave?.(next)
	}
</script>

{#if editing}
	<!-- Every override below exists so the two states are the same size with the text in the same
	     place, and clicking in moves nothing: `!min-h-0` (TextInput floors it at 28px), `!block` (an
	     inline-block textarea baseline-aligns and the idle button does not), and never `!h-auto` —
	     `use:autosize` grows the field with an inline `style.height` an `!important` class beats. -->
	<TextInput
		bind:this={textInputComponent}
		bind:value={draft}
		{size}
		underlyingInputEl="textarea"
		autosizeParams={{ minHeight: 0 }}
		class="!block !bg-transparent !border-0 !shadow-none !m-0 !min-w-0 !min-h-0 !py-0 {textClass} {inputClass} {className}"
		inputProps={{
			placeholder,
			rows: 1,
			onblur: save,
			onkeydown: handleKeydown,
			oninput: commitOnInput ? handleLiveInput : undefined,
			spellcheck: false,
			style: 'padding: 2px !important; resize: none'
		}}
	/>
{:else}
	<!-- Padding 2px over, 4px under: the same 22px box as the textarea *and* the same 2px above the
	     first line. An even 3px gets the box right and leaves the text a pixel low. -->
	<button
		type="button"
		onclick={startEditing}
		disabled={!editable}
		aria-label={editable ? `Edit ${placeholder.toLowerCase() || 'value'}` : undefined}
		class="w-full text-left whitespace-pre-wrap break-words rounded px-0.5 pt-[2px] pb-[4px] {editable
			? 'cursor-text hover:bg-surface-hover'
			: 'cursor-default'} {textClass} {className}"
	>
		{value || placeholder}
	</button>
{/if}
