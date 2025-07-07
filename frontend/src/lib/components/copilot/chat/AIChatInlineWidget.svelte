<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import LoadingIcon from '$lib/components/apps/svelte-select/lib/LoadingIcon.svelte'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy } from 'svelte'
	import type { AIChatEditorHandler } from './monaco-adapter'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		selection: Selection | null
		selectedCode: string
		show: boolean
		rejectChanges: () => void
		editorHandler: AIChatEditorHandler
	}

	let {
		editor,
		selection,
		selectedCode,
		show = $bindable(false),
		rejectChanges,
		editorHandler
	}: Props = $props()

	let widget: AIChatWidget | null = $state(null)
	let widgetElement: HTMLElement | null = $state(null)
	let aiChatInput: AIChatInput | null = $state(null)
	let processing = $state(false)
	let marginToAdd = $state(0)

	class AIChatWidget implements monaco.editor.IContentWidget {
		private domNode: HTMLElement
		public position: monaco.IPosition
		editor: monaco.editor.IStandaloneCodeEditor
		private originalPadding: number = 0
		private editorHandler: AIChatEditorHandler

		constructor(
			lineNumber: number,
			domNode: HTMLElement,
			editor: monaco.editor.IStandaloneCodeEditor,
			editorHandler: AIChatEditorHandler
		) {
			this.domNode = domNode
			this.position = { lineNumber, column: 0 }
			this.editor = editor
			this.editorHandler = editorHandler
			this.ensureSpaceAbove()

			this.editor.onDidScrollChange((e) => {
				this.domNode.style.visibility = 'inherit'
			})
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
					this.editor.updateOptions({
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
					this.editor.updateOptions({
						padding: {
							top: this.originalPadding
						}
					})
					this.editor.layout()
				}
			}
		}

		getId(): string {
			return 'ai-inline-chat-widget'
		}

		getDomNode(): HTMLElement {
			return this.domNode
		}

		getAddedLines(): number {
			if (!this.editorHandler || !this.editorHandler.groupChanges) {
				return 0
			}

			let totalAddedLines = 0
			let totalRemovedLines = 0
			for (const group of this.editorHandler.groupChanges) {
				for (const change of group.changes) {
					if (change.type === 'added_block') {
						// Count newlines in the added content
						const lines = change.value.split('\n').length - 1
						totalAddedLines += Math.max(1, lines)
					} else if (change.type === 'deleted') {
						const lines = change.range.endLine - change.range.startLine + 1
						totalRemovedLines += Math.max(1, lines)
					}
				}
			}
			return totalAddedLines - totalRemovedLines
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
			widget = new AIChatWidget(startLine, widgetElement, editor, editorHandler)
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

	// Reactive effect to update margin when review state changes
	$effect(() => {
		const isInReviewMode = aiChatManager.pendingNewCode !== undefined

		if (isInReviewMode && widget) {
			const addedLines = widget.getAddedLines()
			if (widgetElement) {
				marginToAdd = addedLines * 25
				widgetElement.style.marginTop = `-${marginToAdd}px`
			}
		} else {
			marginToAdd = 0
		}
	})
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
