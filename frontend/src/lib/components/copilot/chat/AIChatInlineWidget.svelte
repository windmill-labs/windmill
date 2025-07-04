<script lang="ts">
	import * as monaco from 'monaco-editor'
	import AIChatInput from './AIChatInput.svelte'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import type { Selection } from 'monaco-editor'
	import LoadingIcon from '$lib/components/apps/svelte-select/lib/LoadingIcon.svelte'

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
			editor.removeContentWidget(widget)
			widget = null
		}
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
					return
				}
				processing = true
				const reply = await aiChatManager.sendInlineRequest(instructions, selectedCode, selection)
				if (reply) {
					aiChatManager.scriptEditorApplyCode?.(reply)
					pendingCode = reply
				}
				processing = false
				focusInput()
			}}
			onKeyDown={(e) => {
				if (e.key === 'Escape') {
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
