<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import SubGridOutput from '../SubGridOutput.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		id: string
		first?: boolean
		label: string
		renderRec: boolean
		numberOfSubgrids?: number
	}

	let { id, first = false, label, renderRec, numberOfSubgrids = 0 }: Props = $props()

	let subGrids = $derived(Array.from({ length: numberOfSubgrids }).map((_, i) => `${id}-${i}`))
</script>

<OutputHeader render={renderRec} renamable={false} {id} name={label} {first}>
	{#snippet children({ render })}
		<ComponentOutputViewer
			{render}
			componentId={id}
			on:select={({ detail }) => {
				$connectingInput = connectInput($connectingInput, id, detail)
			}}
		/>
		{#if subGrids.length > 0}
			<SubGridOutput {render} parentId={id} {subGrids} />
		{/if}
	{/snippet}
</OutputHeader>
