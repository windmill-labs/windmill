<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { artifactPlugins } from './artifactPlugins'
	import ArtifactPrintCodeBlock from './ArtifactPrintCodeBlock.svelte'
	// These load into the app's page, not the print frame: the frame copies the page's stylesheets.
	import '@fontsource-variable/jetbrains-mono/wght.css'
	import './artifactDocument.css'

	let { content }: { content: string } = $props()

	const plugins = [...artifactPlugins, { renderer: { pre: ArtifactPrintCodeBlock } }]
</script>

<div class="wm-doc">
	<Markdown md={content} {plugins} />
</div>
