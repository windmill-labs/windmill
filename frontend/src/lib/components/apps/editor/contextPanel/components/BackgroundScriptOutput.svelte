<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { selectedComponent, connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string
	export let name: string
	export let expanded: boolean = false
	export let first: boolean = false

	function onHeaderClick(manuallyOpen: boolean) {
		if (manuallyOpen) {
			$selectedComponent = undefined
		} else {
			$selectedComponent = id
		}
	}
</script>

<OutputHeader
	{id}
	{name}
	color="blue"
	{first}
	{expanded}
	on:handleClick={(e) => {
		onHeaderClick(e.detail.manuallyOpen)
	}}
	shouldOpen={$selectedComponent === id}
>
	<ComponentOutputViewer
		componentId={id}
		outputs={['loading', 'result']}
		on:select={({ detail }) => {
			$connectingInput = connectInput($connectingInput, id, detail)
		}}
	/>
</OutputHeader>
