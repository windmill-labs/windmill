<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import { slide } from 'svelte/transition'
	import { connectInput } from '../../appUtils'
	import ComponentOutputViewer from '../ComponentOutputViewer.svelte'
	import OutputHeader from './OutputHeader.svelte'

	const { staticOutputs, connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string

	let open: boolean = false
	let manuallyOpen: boolean = false

	$: isOpen = open || manuallyOpen
</script>

<OutputHeader
	open={isOpen}
	{manuallyOpen}
	on:click={() => {
		manuallyOpen = !manuallyOpen
	}}
	{id}
	name={'Table action'}
/>
{#if isOpen}
	<div transition:slide|local>
		<ComponentOutputViewer
			componentId={id}
			outputs={$staticOutputs[id]}
			on:select={({ detail }) => {
				$connectingInput = connectInput($connectingInput, id, detail)
			}}
		/>
	</div>
{/if}
