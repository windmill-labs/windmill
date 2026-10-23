<script lang="ts">
	import { ArrowUp } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button } from '$lib/components/common'
	import autosize from '$lib/autosize'
	import { CHAT_INPUT_PADDING } from './aiChatManagerContext'
	import type { Snippet } from 'svelte'
	import {
		composerBoxClass,
		COMPOSER_FIELD_RESET,
		HERO_BOX_CLASS,
		type ComposerSelection
	} from './composerBox'

	/**
	 * The chat composer's box, drawn while the chat code loads. It must import nothing of
	 * AIChat's: it exists so the page can show and type into the composer before that code
	 * arrives. The host passes `value` and the last selection to the real composer when it mounts.
	 */
	let {
		value = $bindable(),
		placeholder,
		disabled = false,
		autofocus = false,
		onSelectionChange,
		onCompositionChange,
		toolbar
	}: {
		value: string
		placeholder?: string
		disabled?: boolean
		autofocus?: boolean
		/** The caret while the field has focus, undefined once the user moves to another element. */
		onSelectionChange?: (selection: ComposerSelection | undefined) => void
		/** Whether an IME composition is in progress: replacing the field then would lose it. */
		onCompositionChange?: (composing: boolean) => void
		/** Draws the box that opens a session (AIChatInput's `hero`), its controls row in the box. */
		toolbar?: Snippet
	} = $props()

	function trackSelection(e: Event & { currentTarget: HTMLTextAreaElement }) {
		onSelectionChange?.({
			start: e.currentTarget.selectionStart,
			end: e.currentTarget.selectionEnd
		})
	}
</script>

{#snippet sendButton(size: 'sm' | 'md')}
	<Button
		variant="subtle"
		unifiedSize={size}
		iconOnly
		title="Send"
		startIcon={{ icon: ArrowUp }}
		disabled
	/>
{/snippet}

<div class="relative mt-1">
	<div class={twMerge(composerBoxClass(disabled), toolbar && HERO_BOX_CLASS)}>
		<div class="relative w-full">
			<!-- Text metrics match ContextTextarea's `.textarea-input`, so the swap moves nothing. -->
			<textarea
				{@attach (node) => {
					if (autofocus) node.focus()
				}}
				bind:value
				use:autosize={toolbar ? { maxHeight: 160, minHeight: 56 } : { maxHeight: '40vh' }}
				rows={1}
				{placeholder}
				{disabled}
				class={twMerge(
					'w-full resize-none overflow-clip whitespace-pre-wrap break-words text-xs leading-[1.72] min-h-[1.9rem]',
					COMPOSER_FIELD_RESET,
					CHAT_INPUT_PADDING,
					toolbar && '!pl-4 !pr-4 !pt-3 !pb-1'
				)}
				onfocus={trackSelection}
				oninput={trackSelection}
				onselect={trackSelection}
				onkeyup={trackSelection}
				onpointerup={trackSelection}
				onblur={(e) => {
					// Unmounting the focused field blurs it, while still connected; that is the hand-off,
					// not the user leaving. The field is detached by the time a microtask runs. A window
					// blur leaves the field the active element, so it is not the user leaving either.
					const field = e.currentTarget
					queueMicrotask(() => {
						if (field.isConnected && document.activeElement !== field) {
							onSelectionChange?.(undefined)
						}
					})
				}}
				oncompositionstart={() => onCompositionChange?.(true)}
				oncompositionend={() => onCompositionChange?.(false)}
				onkeydown={(e) => {
					// Enter sends in the real composer; here it must not add a newline to the draft.
					if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) e.preventDefault()
				}}
			></textarea>
			{#if !toolbar}
				<div class="absolute bottom-1 right-1">
					{@render sendButton('md')}
				</div>
			{/if}
		</div>
		{#if toolbar}
			<div class="flex flex-row items-center gap-1.5 px-3 pb-1.5">
				<div class="flex-1 min-w-0">{@render toolbar()}</div>
				{@render sendButton('sm')}
			</div>
		{/if}
	</div>
</div>
