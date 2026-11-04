<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { markdownProse } from '$lib/components/markdownProse'
	import { markdownPlugins } from '$lib/components/markdownPlugins'
	import ArtifactPrintCodeBlock from './ArtifactPrintCodeBlock.svelte'

	let { content }: { content: string } = $props()

	const plugins = [...markdownPlugins, { renderer: { pre: ArtifactPrintCodeBlock } }]
</script>

<div class={markdownProse.doc}>
	<Markdown md={content} {plugins} />
</div>
