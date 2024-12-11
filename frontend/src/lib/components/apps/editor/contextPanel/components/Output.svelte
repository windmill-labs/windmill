<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string
	export let first: boolean = false
	export let label: string
	export let renderRec: boolean
</script>

<OutputHeader render={renderRec} let:render renamable={false} {id} name={label} {first}>
	<ComponentOutputViewer
		{render}
		componentId={id}
		on:select={({ detail }) => {
			$connectingInput = connectInput($connectingInput, id, detail)
		}}
	/>
</OutputHeader>
