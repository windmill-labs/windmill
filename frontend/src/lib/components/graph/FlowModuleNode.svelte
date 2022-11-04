<script lang="ts">
	import { getContext } from 'svelte'
	import type { SupportedLanguage } from '$lib/common'
	import LanguageIcon from '../common/languageIcons/LanguageIcon.svelte'
	import FlowModuleHostIcon from './FlowModuleHostIcon.svelte'
	import {
		GraphNode,
		GRAPH_CONTEXT_KEY,
		ellipsize,
		type GraphNodeClass,
		type ModuleHost,
		type GraphContext,
	} from '.'

	const PADDING = 8
	const ICON = 24

	export let node: GraphNodeClass
	export let title: string
	export let lang: SupportedLanguage | undefined = undefined
	export let host: ModuleHost
	const { width } = getContext<GraphContext>(GRAPH_CONTEXT_KEY).node
	let wrapper: SVGGElement
	let nodeType: SVGGElement
	let nodeTitle: SVGTextElement

	$: maxTextWidth = (node.box.width - (nodeType?.getBoundingClientRect().width || 0) - (PADDING * 2))
	$: nodeTitle && ellipsize(title, nodeTitle, maxTextWidth)
</script>

<GraphNode {node}>
	<svelte:fragment slot="background">
		<title>{title}</title>
	</svelte:fragment>
	<g bind:this={wrapper} transform={`translate(${PADDING} ${PADDING})`}>
		<g bind:this={nodeType} transform={`translate(${width - PADDING - 60})`}>
			{#if lang}
				<g transform={`translate(0 ${host === 'hub' ? -1 : 0})`}>
					<LanguageIcon scale={1.5} {lang} class="grayscale" />
				</g>
			{/if}
			<g transform={`translate(${ICON + PADDING * 0.5})`}>
				<FlowModuleHostIcon scale={1.5} {host} class="grayscale" />
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
