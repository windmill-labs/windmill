<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { markdownProse } from '$lib/components/markdownProse'
	import { markdownPlugins } from '$lib/components/markdownPlugins'
	import ArtifactPrintCodeBlock from './ArtifactPrintCodeBlock.svelte'
	import MarkdownImage from '../MarkdownImage.svelte'
	import MarkdownImageSource from '../MarkdownImageSource.svelte'
	import { setUntrustedImages } from '../untrustedImages'
	import { untrack } from 'svelte'

	let { content, untrustedImages = false }: { content: string; untrustedImages?: boolean } =
		$props()

	// Mounted in its own root, so the viewer's context does not reach it: the printing page
	// passes it on, or a shared artifact's outside images load while the PDF is prepared.
	if (untrack(() => untrustedImages)) setUntrustedImages()

	const plugins = [
		...markdownPlugins,
		{ renderer: { pre: ArtifactPrintCodeBlock, img: MarkdownImage, source: MarkdownImageSource } }
	]
</script>

<div class={markdownProse.doc}>
	<Markdown md={content} {plugins} />
</div>
