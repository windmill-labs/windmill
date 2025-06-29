<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		id: string
		first?: boolean
		label: string
		renderRec: boolean
	}

	let { id, first = false, label, renderRec }: Props = $props()
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
	{/snippet}
</OutputHeader>
