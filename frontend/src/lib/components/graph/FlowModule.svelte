<script lang="ts">
	import { getContext } from 'svelte'
	import { select } from 'd3'
	import type { SupportedLanguage } from '$lib/common'
	import LanguageIcon from '../common/languageIcons/LanguageIcon.svelte'
	import FlowModuleHostIcon from './FlowModuleHostIcon.svelte'
	import { GraphNode, type GraphNodeClass, type ModuleHost, type NodeSizeContext } from '.'

	const PADDING = 8
	const ICON = 24

	export let node: GraphNodeClass
	export let title: string
	export let lang: SupportedLanguage
	export let host: ModuleHost
	const { w: WIDTH } = getContext<NodeSizeContext>('nodeSize')
	let nodeType: SVGGElement
	let nodeTitle: SVGTextElement

	$: if (nodeTitle) ellipsize(title)

	const ellipsize = (text: string) => {
		const elem = select(nodeTitle)
		const maxLength = WIDTH - 2 * PADDING - nodeType.getBoundingClientRect().width
		const cutoff = '...'
		let length = (<SVGTextElement>elem.node()).getComputedTextLength()
		if (!length || length < maxLength) {
			elem.append('title').text(title)
			return
		}

		text = cutoff + text
		while (length > maxLength && text.length) {
			text = text.slice(0, -1)
			elem.text(text.slice(cutoff.length) + cutoff)
			elem.append('title').text(title)
			length = (<SVGTextElement>elem.node()).getComputedTextLength()
		}
		elem.text(text.slice(cutoff.length).trimEnd() + cutoff)
	}
</script>

<GraphNode {node}>
	<svelte:fragment slot="background">
		<title>{title}</title>
	</svelte:fragment>
	<g transform={`translate(${PADDING} ${PADDING})`}>
		<g bind:this={nodeType} transform={`translate(${WIDTH - PADDING - 60})`}>
			<g transform={`translate(0 ${host === 'hub' ? -1 : 0})`}>
				<FlowModuleHostIcon scale={1.5} {host} class="grayscale" />
			</g>
			<g transform={`translate(${ICON + PADDING * 0.5})`}>
				<LanguageIcon scale={1.5} {lang} class="grayscale" />
			</g>
		</g>
		{#if nodeType}
			<text
				bind:this={nodeTitle}
				transform="translate(0 17)"
				fill="#111827"
				class="select-none text-sm font-medium"
			>
				{title}
			</text>
		{/if}
	</g>
</GraphNode>
