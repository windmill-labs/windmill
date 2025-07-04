<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import LoadingIcon from '$lib/components/apps/svelte-select/lib/LoadingIcon.svelte'
	import { onDestroy } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		selection: Selection | null
		selectedCode: string
		show: boolean
	}

	let { editor, selection, selectedCode, show = $bindable(false) }: Props = $props()

	let widget: AIChatWidget | null = $state(null)
	let widgetElement: HTMLElement | null = $state(null)
	let aiChatInput: AIChatInput | null = $state(null)
	let processing = $state(false)
	let pendingCode = $state('')

	class AIChatWidget implements monaco.editor.IContentWidget {
		private domNode: HTMLElement
		public position: monaco.IPosition

		constructor(lineNumber: number, domNode: HTMLElement) {
			this.domNode = domNode
			this.position = { lineNumber, column: 0 }
		}

		getId(): string {
			return 'simple-ai-widget'
		}

		getDomNode(): HTMLElement {
			return this.domNode
		}

		getPosition(): monaco.editor.IContentWidgetPosition {
			return {
				position: this.position,
				preference: [
					monaco.editor.ContentWidgetPositionPreference.ABOVE,
					monaco.editor.ContentWidgetPositionPreference.BELOW
				]
			}
		}
	}

	// Cleanup function to safely remove widget and cancel requests
	function cleanupWidget() {
		// Cancel any ongoing request
		if (processing && aiChatManager.abortController && !aiChatManager.abortController.signal.aborted) {
			console.log('Cancelling ongoing request during cleanup')
			aiChatManager.abortController.abort()
		}
		
		if (widget) {
			try {
				editor.removeContentWidget(widget)
			} catch (error) {
				console.warn('Failed to remove content widget:', error)
			}
			widget = null
		}
		
		// Reset processing state
		processing = false
		pendingCode = ''
	}

	// Create/remove widget based on show state
	$effect(() => {
		if (show && !widget && widgetElement && selection) {
			if (aiChatManager.mode !== AIMode.SCRIPT) {
				aiChatManager.changeMode(AIMode.SCRIPT)
			}
			const startLine = selection.startLineNumber
			widget = new AIChatWidget(startLine, widgetElement)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			aiChatManager.cancel()
			cleanupWidget()
		}

		// Cleanup function for the effect
		return () => {
			if (widget) {
				cleanupWidget()
			}
		}
	})

	// Ensure widget is cleaned up when component is destroyed
	onDestroy(() => {
		cleanupWidget()
	})

	$effect(() => {
		if (!aiChatManager.pendingNewCode && pendingCode) {
			pendingCode = ''
		}
	})

	export function focusInput() {
		setTimeout(() => {
			aiChatInput?.focusInput()
		}, 130)
	}
</script>

{#snippet bottomRightSnippet()}
	{#if processing}
		<LoadingIcon />
	{:else if pendingCode}
		<span class="text-xs text-tertiary pr-1">↓ to apply</span>
	{:else}
		<div></div>
	{/if}
{/snippet}

{#if show}
	<div bind:this={widgetElement} class="w-[300px] -mt-2">
		<AIChatInput
			bind:this={aiChatInput}
			availableContext={aiChatManager.contextManager.getAvailableContext()}
			selectedContext={aiChatManager.contextManager.getSelectedContext()}
			onClickOutside={() => {
				show = false
			}}
			onSendRequest={async (instructions) => {
				if (!selection) {
					sendUserToast('No code selected', true)
					return
				}
				
				// Prevent multiple concurrent requests
				if (processing) {
					console.log('Request already in progress, ignoring duplicate request')
					return
				}
				
				// Cancel any ongoing request before starting a new one
				if (aiChatManager.abortController && !aiChatManager.abortController.signal.aborted) {
					console.log('Cancelling previous request')
					aiChatManager.abortController.abort()
				}
				
				processing = true
				
				try {
					const reply = await aiChatManager.sendInlineRequest(instructions, selectedCode, selection)
					if (reply) {
						aiChatManager.scriptEditorApplyCode?.(reply)
						pendingCode = reply
					}
				} catch (error) {
					console.error('Inline AI request failed:', error)
					if (error instanceof Error) {
						// Don't show error toast for cancelled requests
						if (error.message !== 'Request was cancelled') {
							sendUserToast('AI request failed: ' + error.message, true)
						}
					} else {
						sendUserToast('AI request failed: Unknown error', true)
					}
				} finally {
					processing = false
				}
				
				focusInput()
			}}
			onKeyDown={(e) => {
				if (e.key === 'Escape') {
					// Cancel any ongoing request
					if (processing && aiChatManager.abortController && !aiChatManager.abortController.signal.aborted) {
						console.log('Cancelling request due to Escape key')
						aiChatManager.abortController.abort()
					}
					show = false
				} else if (e.key === 'ArrowDown' && pendingCode) {
					// call again to auto apply
					aiChatManager.scriptEditorApplyCode?.(pendingCode)
					pendingCode = ''
					show = false
				}
			}}
			showContext={false}
			className="-ml-2"
			bottomRightSnippet={processing || pendingCode ? bottomRightSnippet : undefined}
			disabled={processing}
		/>
	</div>
{/if}
