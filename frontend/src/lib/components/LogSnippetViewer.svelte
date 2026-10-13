<script lang="ts">
	import { untrack } from 'svelte'
	import { AnsiUp } from 'ansi_up'

	interface Props {
		content: string
		highlighted: any[]
		onClick?: () => void
	}

	let { content, highlighted, onClick }: Props = $props()

	const ansi_up = new AnsiUp()
	ansi_up.use_classes = true

	function highlightSnippet(snippet: string) {
		const opener = '!-!-!_H_START_WMILL_!-!-!'
		const closer = '!-!-!_H_ENDER_WMILL_!-!-!'

		let offset = 0
		let ret = snippet
		for (const range of highlighted) {
			ret = ret.slice(0, range.start + offset) + opener + ret.slice(range.start + offset)
			offset += opener.length
			ret = ret.slice(0, range.end + offset) + closer + ret.slice(range.end + offset)
			offset += closer.length
		}

		let html = ansi_up.ansi_to_html(ret)
		let html2 = html
			.replaceAll(opener, '<span class="bg-amber-400 text-black">')
			.replaceAll(closer, '</span>')
		return html2
	}

	let html = highlightSnippet(untrack(() => content))
</script>

<button
	type="button"
	onclick={onClick}
	title="Show in context"
	class="block w-full min-w-full px-3 py-0.5 text-left font-mono text-2xs text-primary whitespace-pre hover:bg-surface-hover focus-visible:bg-surface-hover focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-border-accent"
	>{@html html}</button
>
