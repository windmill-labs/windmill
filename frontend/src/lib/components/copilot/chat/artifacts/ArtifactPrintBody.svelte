<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { artifactPlugins } from './artifactPlugins'
	import ArtifactPrintCodeBlock from './ArtifactPrintCodeBlock.svelte'
	import MarkdownImage from '../MarkdownImage.svelte'
	import MarkdownImageSource from '../MarkdownImageSource.svelte'
	import { setUntrustedImages } from '../untrustedImages'
	import { untrack } from 'svelte'
	// These load into the app's page, not the print frame: the frame copies the page's stylesheets.
	import '@fontsource-variable/jetbrains-mono/wght.css'
	import './artifactDocument.css'

	let { content, untrustedImages = false }: { content: string; untrustedImages?: boolean } =
		$props()

	// Mounted in its own root, so the viewer's context does not reach it: the printing page
	// passes it on, or a shared artifact's outside images load while the PDF is prepared.
	if (untrack(() => untrustedImages)) setUntrustedImages()

	const plugins = [
		...artifactPlugins,
		{ renderer: { pre: ArtifactPrintCodeBlock, img: MarkdownImage, source: MarkdownImageSource } }
	]
</script>

<div class="wm-doc">
	<Markdown md={content} {plugins} />
</div>
