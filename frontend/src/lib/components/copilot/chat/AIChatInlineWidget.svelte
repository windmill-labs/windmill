<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'

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
		console.log('show', show)
		if (show && !widget && widgetElement && selection) {
			console.log('adding widget', selection)
			const startLine = selection.startLineNumber
			widget = new SimpleContentWidget(startLine, widgetElement)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			console.log('removing widget', selection)
			editor.removeContentWidget(widget)
			widget = null
		}
	})

	export function focusInput() {
		console.log('focusing input')
		aiChatInput?.focusInput()
	}
</script>

{#if show}
	<div bind:this={widgetElement} class="w-[300px]">
		<AIChatInput
			bind:this={aiChatInput}
			availableContext={aiChatManager.contextManager.getAvailableContext()}
			selectedContext={aiChatManager.contextManager.getSelectedContext()}
			onClickOutside={() => {
				show = false
			}}
			onSendRequest={async (instructions) => {
				console.log('sending request', instructions)
				const reply = await aiChatManager.sendInlineRequest(instructions, selectedCode, selection)
				aiChatManager.scriptEditorApplyCode?.(reply)
			}}
			className="-ml-2"
			showContext={false}
		/>
	</div>
{/if}
