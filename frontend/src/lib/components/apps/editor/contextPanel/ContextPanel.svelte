<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { X } from 'lucide-svelte'
	import { getContext, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { slide } from 'svelte/transition'

	import type { AppViewerContext } from '../../types'
	import { connectInput, sortGridItemsPosition } from '../appUtils'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'
	import MinMaxButton from './components/MinMaxButton.svelte'
	import OutputHeader from './components/OutputHeader.svelte'

	const { connectingInput, breakpoint, app, state } =
		getContext<AppViewerContext>('AppViewerContext')

	function toggleExpanded() {
		expanded = !expanded
	}

	let search = writable<string>('')
	let expanded = false
	let ctxOpened = true

	setContext('searchCtx', {
		search
	})

	$: expanded && !ctxOpened && (ctxOpened = true)

	/**
	$: panels = [['ctx', ['email', 'username', 'query', 'hash']] as [string, string[]]].concat(
		Object.entries($staticOutputs)
	)

	// filter out outputs that don't match the search by name (computed by getComponentNameById) and id
	// The output should be [string, string[]][]
	$: filteredPanels = panels.filter(([componentId, outputs]) => {
		const name = getComponentNameById(componentId)
		return (
			name.toLowerCase().includes(search.toLowerCase()) ||
			componentId.toLowerCase().includes(search.toLowerCase())
		)

	})
	*/
</script>

<PanelSection noPadding titlePadding="px-4 pt-2 pb-0.5" title="Outputs">
	<div style="z-index:1000;" class="bg-white">
		<div class="overflow-auto h-full min-w-[150px] w-full relative flex flex-col">
			<div class="sticky z-50 top-0 left-0 w-full bg-white p-2">
				<div class="relative">
					<input
						bind:value={$search}
						class="px-2 py-1 border border-gray-300 rounded-sm {search ? 'pr-8' : ''}"
						placeholder="Search outputs..."
					/>
					{#if search}
						<button
							class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-200 rounded-full p-0.5"
							on:click|stopPropagation|preventDefault={() => ($search = '')}
						>
							<X size="14" />
						</button>
					{/if}
				</div>
			</div>

			<div class="p-1 ">
				<MinMaxButton on:click={toggleExpanded} {expanded} />
			</div>

			<OutputHeader
				open={ctxOpened}
				manuallyOpen={false}
				on:click={() => {
					ctxOpened = !ctxOpened
				}}
				id={'ctx'}
				name={'App Context'}
				first={true}
				nested={true}
			/>

			{#if ctxOpened}
				<div class="my-1" transition:slide|local>
					<ComponentOutputViewer
						componentId={'ctx'}
						outputs={['email', 'username', 'query', 'hash']}
						on:select={({ detail }) => {
							$connectingInput = connectInput($connectingInput, 'ctx', detail)
						}}
					/>
				</div>
			{/if}

			{#each sortGridItemsPosition($app.grid, $breakpoint) as gridItem, index}
				<ComponentOutput {gridItem} first={index === 0} {expanded} />
			{/each}
		</div>
		<PanelSection noPadding titlePadding="px-4 pt-2 pb-0.5" title="State">
			<div class="mx-2 px-1 border w-full mb-8">
				{#key $state}
					<ObjectViewer json={$state} />
				{/key}
			</div>
		</PanelSection>
	</div>
</PanelSection>
