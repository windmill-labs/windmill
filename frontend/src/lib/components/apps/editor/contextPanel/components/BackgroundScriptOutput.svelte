<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { selectedComponent, connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string
	export let name: string
	export let first: boolean = false

	function onHeaderClick(manuallyOpen: boolean) {
		if (manuallyOpen) {
			if (id) {
				$selectedComponent = [id]
			} else {
				$selectedComponent = undefined
			}
		} else {
			$selectedComponent = [id]
		}
	}
</script>

<OutputHeader
	{id}
	{name}
	color="blue"
	{first}
	on:handleClick={(e) => {
		if (!$connectingInput.opened) {
			onHeaderClick(e.detail.manuallyOpen)
		}
	}}
>
	<ComponentOutputViewer
		componentId={id}
		on:select={({ detail }) => {
			$connectingInput = connectInput($connectingInput, id, detail)
		}}
	/>
</OutputHeader>
