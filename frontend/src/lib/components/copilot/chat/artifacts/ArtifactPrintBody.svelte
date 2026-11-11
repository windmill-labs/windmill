<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { markdownPlugins } from '$lib/components/markdownPlugins'
	import ArtifactPrintCodeBlock from './ArtifactPrintCodeBlock.svelte'
	// These load into the app's page, not the print frame: the frame copies the page's stylesheets.
	import '@fontsource-variable/newsreader/opsz.css'
	import '@fontsource-variable/newsreader/opsz-italic.css'
	import '@fontsource-variable/jetbrains-mono/wght.css'
	import './artifactDocument.css'

	let { content }: { content: string } = $props()

	const plugins = [...markdownPlugins, { renderer: { pre: ArtifactPrintCodeBlock } }]
</script>

<div class="wm-doc">
	<Markdown md={content} {plugins} />
</div>
