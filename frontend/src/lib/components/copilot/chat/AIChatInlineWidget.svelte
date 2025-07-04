<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager } from './AIChatManager.svelte'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		lineNumber: number
		show: boolean
	}

	let { editor, lineNumber, show = $bindable(false) }: Props = $props()

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
		if (show && !widget && widgetElement) {
			console.log('adding widget', lineNumber)
			widget = new SimpleContentWidget(lineNumber, widgetElement)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			console.log('removing widget', lineNumber)
			editor.removeContentWidget(widget)
			widget = null
			isPositionedBelow = false
		}
	})

	export function focusInput() {
		console.log('focusing input')
		aiChatInput?.focusInput()
	}
</script>

<div bind:this={widgetElement} class="w-[300px]">
	<AIChatInput
		bind:this={aiChatInput}
		availableContext={aiChatManager.contextManager.getAvailableContext()}
		selectedContext={aiChatManager.contextManager.getSelectedContext()}
		onClickOutside={() => {
			show = false
		}}
		onSendRequest={(instructions) => {
			console.log('sending request', instructions)
			aiChatManager.sendInlineRequest(instructions)
		}}
		className="-ml-2"
	/>
</div>
