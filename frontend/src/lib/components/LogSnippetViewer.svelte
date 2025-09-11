<script lang="ts">
	import { AnsiUp } from 'ansi_up'

	export let content: string
	export let highlighted: any[]

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

	let html = highlightSnippet(content)
</script>

<button on:click class="font-light !m-0 !p-0">
	<pre
		class="bg-surface-secondary hover:bg-surface px-2 py-1 text-secondary text-xs w-[100%] whitespace-pre border min-w-full text-start">
{@html html}
</pre>
</button>
