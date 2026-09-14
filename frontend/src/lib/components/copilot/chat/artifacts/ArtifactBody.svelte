<script lang="ts">
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { markdownProse } from '$lib/components/markdownProse'
	import CodeDisplay from '../script/CodeDisplay.svelte'
	import LinkRenderer from '../LinkRenderer.svelte'

	interface Props {
		content: string
		/** The raw text rather than the rendered document. */
		source: boolean
	}

	let { content, source }: Props = $props()

	const plugins = [gfmPlugin(), { renderer: { pre: CodeDisplay, a: LinkRenderer } }]
</script>

<!-- Rendered inside the caller's scroll container: the fade below is sticky to it. -->
{#if source}
	<!-- key: SimpleEditor reads `code` only on init. -->
	{#key content}
		<SimpleEditor lang="markdown" code={content} readOnly class="h-full" />
	{/key}
{:else}
	<!-- Pinned under the header, fades scrolled-under content instead of hard-clipping it.
	     The negative margin cancels its flow height so it overlays instead of pushing. -->
	<div class="sticky top-0 z-10 h-4 -mb-4 bg-gradient-to-b from-surface-tertiary to-transparent"
	></div>
	<div class="pb-4 pt-2 {markdownProse.doc}">
		<Markdown md={content} {plugins} />
	</div>
{/if}
