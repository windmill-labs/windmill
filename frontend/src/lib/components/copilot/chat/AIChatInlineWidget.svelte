<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import LoadingIcon from '$lib/components/apps/svelte-select/lib/LoadingIcon.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy } from 'svelte'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		selection: Selection | null
		selectedCode: string
		show: boolean
		rejectChanges: () => void
	}

	let { editor, selection, selectedCode, show = $bindable(false), rejectChanges }: Props = $props()

	let widget: AIChatWidget | null = $state(null)
	let widgetElement: HTMLElement | null = $state(null)
	let aiChatInput: AIChatInput | null = $state(null)
	let processing = $state(false)

	class AIChatWidget implements monaco.editor.IContentWidget {
		private domNode: HTMLElement
		public position: monaco.IPosition
		private editor: monaco.editor.IStandaloneCodeEditor
		private originalPadding: number = 0

		constructor(
			lineNumber: number,
			domNode: HTMLElement,
			editor: monaco.editor.IStandaloneCodeEditor
		) {
			this.domNode = domNode
			this.position = { lineNumber, column: 0 }
			this.editor = editor
			this.ensureSpaceAbove()
		}

		private ensureSpaceAbove() {
			const widgetHeight = 100
			const additionalPadding = 15
			const linesHeight = 20 * this.position.lineNumber
			if (linesHeight < widgetHeight + additionalPadding) {
				// Add top margin to editor to make space
				const editorDom = this.editor.getDomNode()
				if (editorDom) {
					const neededPadding = widgetHeight + additionalPadding - linesHeight
					this.originalPadding = this.editor.getOption(monaco.editor.EditorOption.padding)?.top || 0
					editor.updateOptions({
						padding: {
							top: neededPadding
						}
					})
					// Trigger layout update
					this.editor.layout()
				}
			}
		}

		private restoreEditorSpacing() {
			if (this.position.lineNumber < 10) {
				const editorDom = this.editor.getDomNode()
				if (editorDom) {
					editor.updateOptions({
						padding: {
							top: this.originalPadding
						}
					})
					this.editor.layout()
				}
			}
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
				preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE]
			}
		}

		dispose() {
			this.restoreEditorSpacing()
		}
	}

	// Cleanup function to safely remove widget and cancel requests
	function cleanupWidget() {
		aiChatManager.cancel()
		if (widget) {
			try {
				widget.dispose()
				editor.removeContentWidget(widget)
			} catch (error) {
				console.warn('Failed to remove content widget:', error)
			}
			widget = null
		}
	}

	// Create/remove widget based on show state
	$effect(() => {
		if (show && !widget && widgetElement && selection) {
			if (aiChatManager.mode !== AIMode.SCRIPT) {
				aiChatManager.changeMode(AIMode.SCRIPT)
			}
			const startLine = selection.startLineNumber
			widget = new AIChatWidget(startLine, widgetElement, editor)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			cleanupWidget()
		}
	})

	onDestroy(() => {
		cleanupWidget()
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
	{:else if aiChatManager.pendingNewCode}
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
				if (!selection || processing) {
					return
				}

				processing = true

				try {
					const reply = await aiChatManager.sendInlineRequest(instructions, selectedCode, selection)
					if (reply) {
						aiChatManager.scriptEditorApplyCode?.(reply)
					}
				} catch (error) {
					console.error('Inline AI request failed:', error)
					if (error instanceof Error) {
						sendUserToast('AI request failed: ' + error.message, true)
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
					show = false
					rejectChanges()
				} else if (e.key === 'ArrowDown' && aiChatManager.pendingNewCode) {
					// call again to auto apply
					aiChatManager.scriptEditorApplyCode?.(aiChatManager.pendingNewCode)
					show = false
				}
			}}
			showContext={false}
			className="-ml-2"
			bottomRightSnippet={processing || aiChatManager.pendingNewCode
				? bottomRightSnippet
				: undefined}
			disabled={processing}
		/>
	</div>
{/if}
