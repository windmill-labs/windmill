<script lang="ts">
	import MarkdownCodeBlock from '$lib/components/MarkdownCodeBlock.svelte'
	import { hasUntrustedImages } from '../untrustedImages'

	// Someone else's artifact keeps mermaid off, as its viewer does: a diagram can fetch images
	// from other hosts without going through MarkdownImage.
	const renderMermaid = !hasUntrustedImages()
</script>

<!-- The viewer's code block without the chat context its own renderer needs. Display blocks are
     on as in the viewer. -->
<MarkdownCodeBlock {renderMermaid} lightMermaid renderDisplay staticDisplay />
