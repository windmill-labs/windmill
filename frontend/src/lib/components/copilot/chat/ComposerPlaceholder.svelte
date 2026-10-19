<script lang="ts">
	import { ArrowUp } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { Button } from '$lib/components/common'
	import autosize from '$lib/autosize'
	import { CHAT_INPUT_PADDING } from './aiChatManagerContext'
	import { composerBoxClass, COMPOSER_FIELD_RESET, type ComposerSelection } from './composerBox'

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
		onSelectionChange
	}: {
		value: string
		placeholder?: string
		disabled?: boolean
		autofocus?: boolean
		/** The caret while the field has focus, undefined once the user leaves it. */
		onSelectionChange?: (selection: ComposerSelection | undefined) => void
	} = $props()

	function trackSelection(e: Event & { currentTarget: HTMLTextAreaElement }) {
		onSelectionChange?.({
			start: e.currentTarget.selectionStart,
			end: e.currentTarget.selectionEnd
		})
	}
</script>

<div class="relative mt-1">
	<div class={composerBoxClass(disabled)}>
		<div class="relative w-full">
			<!-- Text metrics match ContextTextarea's `.textarea-input`, so the swap moves nothing. -->
			<textarea
				{@attach (node) => {
					if (autofocus) node.focus()
				}}
				bind:value
				use:autosize={{ maxHeight: '40vh' }}
				rows={1}
				{placeholder}
				{disabled}
				class={twMerge(
					'w-full resize-none overflow-clip whitespace-pre-wrap break-words text-xs leading-[1.72] min-h-[1.9rem]',
					COMPOSER_FIELD_RESET,
					CHAT_INPUT_PADDING
				)}
				onfocus={trackSelection}
				oninput={trackSelection}
				onselect={trackSelection}
				onkeyup={trackSelection}
				onpointerup={trackSelection}
				onblur={(e) => {
					// Unmounting the focused field blurs it, while still connected; that is the hand-off,
					// not the user leaving. The field is detached by the time a microtask runs.
					const field = e.currentTarget
					queueMicrotask(() => {
						if (field.isConnected) onSelectionChange?.(undefined)
					})
				}}
				onkeydown={(e) => {
					// Enter sends in the real composer; here it must not add a newline to the draft.
					if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) e.preventDefault()
				}}
			></textarea>
			<div class="absolute bottom-1 right-1">
				<Button
					variant="subtle"
					unifiedSize="md"
					iconOnly
					title="Send"
					startIcon={{ icon: ArrowUp }}
					disabled
				/>
			</div>
		</div>
	</div>
</div>
