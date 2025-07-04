<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import LoadingIcon from '$lib/components/apps/svelte-select/lib/LoadingIcon.svelte'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		selection: Selection | null
		selectedCode: string
		show: boolean
	}

	let { editor, selection, selectedCode, show = $bindable(false) }: Props = $props()

	let widget: SimpleContentWidget | null = $state(null)
	let widgetElement: HTMLElement | null = $state(null)
	let aiChatInput: AIChatInput | null = $state(null)
	let processing = $state(false)

	class SimpleContentWidget implements monaco.editor.IContentWidget {
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

	// Create/remove widget based on show state
	$effect(() => {
		if (show && !widget && widgetElement && selection) {
			const startLine = selection.startLineNumber
			widget = new SimpleContentWidget(startLine, widgetElement)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			editor.removeContentWidget(widget)
			widget = null
		}
	})

	export function focusInput() {
		setTimeout(() => {
			aiChatInput?.focusInput()
		}, 130)
	}
</script>

{#snippet bottomRightSnippet()}
	<LoadingIcon />
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
					return
				}
				processing = true
				const reply = await aiChatManager.sendInlineRequest(instructions, selectedCode, selection)
				aiChatManager.scriptEditorApplyCode?.(reply)
				processing = false
			}}
			showContext={false}
			className="-ml-2"
			bottomRightSnippet={processing ? bottomRightSnippet : undefined}
			disabled={processing}
		/>
	</div>
{/if}
