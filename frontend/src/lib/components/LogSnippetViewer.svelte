<script lang="ts">
	import AnsiUp from 'ansi_up'

	export let content: string
	export let searchQuery

	const ansi_up = new AnsiUp()
	ansi_up.use_classes = true

	// let 	updated_string = content.replace("&lt;b&gt;", '<span class="search-highlight">').replace("&lt;/b&gt;", '</span>')

	let html = ansi_up.ansi_to_html(content)
	console.log(1, html)
	let html2 = html
		.replaceAll('&lt;b&gt;', '<span class="bg-amber-400 text-black">')
		.replaceAll('&lt;/b&gt;', '</span>')
	console.log(2, html2)
</script>

<pre
	class="bg-surface-secondary text-secondary text-xs w-full p-2 whitespace-pre-wrap border rounded-md">

{@html html2}
</pre>
...See more
<br>
<br>
