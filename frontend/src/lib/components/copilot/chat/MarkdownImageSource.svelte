<script lang="ts">
	import { hasUntrustedImages } from './untrustedImages'

	type Props = { srcSet?: string; srcset?: string; media?: string; type?: string }
	let { srcSet, srcset, media, type }: Props = $props()

	// A `<picture>` picks the first `<source>` that matches over its `<img>`, so a source left in
	// untrusted markup would load an outside image past MarkdownImage's check of the `<img>`.
	const untrusted = hasUntrustedImages()
</script>

{#if !untrusted}
	<source srcset={srcSet ?? srcset} {media} {type} />
{/if}
