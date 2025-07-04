<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		position: monaco.IPosition
		show: boolean
	}

	let { editor, position, show = $bindable(false) }: Props = $props()

	let widget: SimpleContentWidget | null = $state(null)
	let widgetElement: HTMLElement | null = $state(null)
	let aiChatInput: AIChatInput | null = $state(null)
	let isPositionedBelow = $state(false)

	export class SimpleContentWidget implements monaco.editor.IContentWidget {
		private domNode: HTMLElement
		public position: monaco.IPosition

		constructor(position: monaco.IPosition, domNode: HTMLElement) {
			this.domNode = domNode
			this.position = position
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
		if (show && !widget && widgetElement) {
			console.log('adding widget')
			widget = new SimpleContentWidget(position, widgetElement)
			editor.addContentWidget(widget)
			if (aiChatInput) {
				aiChatInput.focusInput()
			}
		} else if (!show && widget) {
			console.log('removing widget')
			editor.removeContentWidget(widget)
			widget = null
			isPositionedBelow = false
		}
	})

	let lastPosition = $state<monaco.IPosition | null>(null)
	$effect(() => {
		if (
			widget &&
			position &&
			(!lastPosition ||
				lastPosition.lineNumber !== position.lineNumber ||
				lastPosition.column !== position.column)
		) {
			widget.position = position
			editor.layoutContentWidget(widget)
			lastPosition = { ...position }
		}
	})
</script>

<div bind:this={widgetElement} class="flex min-w-[300px]">
	<AIChatInput
		bind:this={aiChatInput}
		availableContext={[]}
		selectedContext={[]}
		showContext={false}
		className="!px-0 mb-2"
		onClickOutside={() => {
			show = false
		}}
	/>
</div>
