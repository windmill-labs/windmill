<script lang="ts">
	import * as monaco from 'monaco-editor'
	import { clickOutside } from '$lib/utils'
	import InlinePrompt from './InlinePrompt.svelte'

	interface Props {
		editor: monaco.editor.IStandaloneCodeEditor
		/** The line the box sits over. */
		line: number
		/** Resolving to `false` keeps the box open with its text. */
		onSend: (prompt: string) => Promise<boolean>
		/** `refocus` unless closing on a click, which already put the focus where the user wants it. */
		onClose: (refocus: boolean) => void
	}

	let { editor, line, onSend, onClose }: Props = $props()

	let widgetElement: HTMLElement | null = $state(null)
	let prompt: InlinePrompt | undefined = $state()

	$effect(() => {
		const domNode = widgetElement
		if (!domNode) return
		const widget: monaco.editor.IContentWidget = {
			// Monaco's overflow layer, which draws above the buttons hosts float over the editor.
			allowEditorOverflow: true,
			getId: () => 'ai-inline-chat-widget',
			getDomNode: () => domNode,
			getPosition: () => ({
				position: { lineNumber: line, column: 1 },
				preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE]
			})
		}
		const padding = editor.getOption(monaco.editor.EditorOption.padding).top
		// Monaco cuts off a widget taller than what is above its line.
		const room = domNode.offsetHeight + 16
		const missing = room - editor.getTopForLineNumber(line)
		if (missing > 0) editor.updateOptions({ padding: { top: padding + missing } })
		// Nor does it draw one whose line is out of view.
		const top = editor.getTopForLineNumber(line)
		const bottom = top + editor.getOption(monaco.editor.EditorOption.lineHeight)
		const lowest = bottom - editor.getLayoutInfo().height
		editor.setScrollTop(Math.min(Math.max(editor.getScrollTop(), lowest), top - room))
		editor.addContentWidget(widget)
		// That layer is not cut at the editor's edge: scrolled up, the box would cover its toolbar.
		const scroll = editor.onDidScrollChange(() => {
			const above = editor.getTopForLineNumber(line) - editor.getScrollTop()
			const hidden = above < domNode.offsetHeight
			domNode.classList.toggle('opacity-0', hidden)
			domNode.classList.toggle('pointer-events-none', hidden)
		})
		// Monaco places the widget, and takes its focus with it, on its next render.
		const timeout = setTimeout(() => prompt?.focus(), 130)
		return () => {
			clearTimeout(timeout)
			scroll.dispose()
			editor.updateOptions({ padding: { top: padding } })
			editor.removeContentWidget(widget)
		}
	})
</script>

<div
	bind:this={widgetElement}
	class="w-[300px] -mt-2 z-20"
	use:clickOutside={{ onClickOutside: () => onClose(false) }}
>
	<InlinePrompt
		bind:this={prompt}
		ariaLabel="Prompt about the selected code"
		{onSend}
		onClose={() => onClose(true)}
	/>
</div>
