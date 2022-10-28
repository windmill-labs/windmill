<script lang="ts">
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import { select, ZoomTransform } from 'd3'
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
	const transform = getContext<Writable<ZoomTransform>>('transform')
	const { w: WIDTH } = getContext<NodeSizeContext>('nodeSize')
	let wrapper: SVGGElement
	let nodeType: SVGGElement
	let nodeTitle: SVGTextElement

	$: if (nodeTitle && title) ellipsize(title)

	const ellipsize = (text: string) => {
		const elem = select(nodeTitle)
		elem.attr('title', title)
		const maxWidth = wrapper.getBoundingClientRect().width - nodeType.getBoundingClientRect().width - (PADDING * 2)
		const cutoff = '...'
		let currentWidth = (<SVGTextElement>elem.text(text).node()).getBoundingClientRect().width
		if (!currentWidth || currentWidth < maxWidth) {
			return
		}

		const cl = cutoff.length
		text = cutoff + text
		while ((currentWidth > maxWidth) && text.length) {
			text = text.slice(0, -1).trimEnd()
			elem.text(text.slice(cl) + cutoff)
			currentWidth = (<SVGTextElement>elem.node()).getBoundingClientRect().width
		}
	}
</script>

<GraphNode {node}>
	<svelte:fragment slot="background">
		<title>{title}</title>
	</svelte:fragment>
	<g bind:this={wrapper} transform={`translate(${PADDING} ${PADDING})`}>
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
				t
			</text>
		{/if}
	</g>
</GraphNode>
