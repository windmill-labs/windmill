<script lang="ts">
	import type { Component } from 'svelte'
	import Markdown from 'svelte-exmarkdown'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import LinkRenderer from '../LinkRenderer.svelte'
	import { artifactPlugins } from './artifactPlugins'
	import '@fontsource-variable/jetbrains-mono/wght.css'
	import './artifactDocument.css'

	interface Props {
		content: string
		/** The raw text rather than the rendered document. */
		source: boolean
		/**
		 * The fenced-code renderer. Defaults to the shared chain's, which keeps mermaid off for
		 * text that can reach readers outside the workspace (a public shared session). The
		 * members-only shared artifact page passes MermaidCodeBlock; the session viewer passes
		 * the chat's, which also needs the chat context the shared pages do not have.
		 */
		pre?: Component<any>
	}

	let { content, source, pre }: Props = $props()

	// The shared chain (raw HTML re-parsed, then sanitized) rather than a chat-only one, because
	// this body also renders what another member wrote. Renderers merge in order, so the chat's
	// link pill and, when given, its code block sit on top of it.
	const plugins = $derived([
		...artifactPlugins,
		{ renderer: { a: LinkRenderer, ...(pre ? { pre } : {}) } }
	])
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
	<div class="wm-doc-frame">
		<div class="wm-doc pb-16 pt-8">
			<Markdown md={content} {plugins} />
		</div>
	</div>
{/if}
