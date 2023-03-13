<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { Maximize, Minimize, X } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import type { AppViewerContext } from '../../types'
	import { connectInput, findGridItem, sortGridItemsPosition } from '../appUtils'
	import { components } from '../component'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	const { staticOutputs, app, breakpoint, connectingInput, selectedComponent } =
		getContext<AppViewerContext>('AppViewerContext')

	function getComponentNameById(componentId: string) {
		const component = findGridItem($app, componentId)

		if (component?.data?.type) {
			return components[component?.data.type].name
		} else if (componentId == 'ctx') {
			return 'Context'
		} else if (componentId.startsWith('bg_')) {
			return 'Background'
		} else {
			return 'Table action'
		}
	}

	function toggleExpanded() {
		expanded = !expanded
	}

	let search = ''
	let expanded = false

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
</script>

<PanelSection noPadding titlePadding="px-4 pt-2 pb-0.5" title="Outputs">
	<div style="z-index:1000;" class="bg-white">
		<div class="overflow-auto h-full min-w-[150px] w-full relative flex flex-col">
			<div class="sticky z-50 top-0 left-0 w-full bg-white px-2 pb-2">
				<div class="relative">
					<input
						bind:value={search}
						class="px-2 py-1 border border-gray-300 rounded-sm {search ? 'pr-8' : ''}"
						placeholder="Search outputs..."
					/>
					{#if search}
						<button
							class="absolute right-2 top-1/2 transform -translate-y-1/2 hover:bg-gray-200 rounded-full p-0.5"
							on:click|stopPropagation|preventDefault={() => (search = '')}
						>
							<X size="14" />
						</button>
					{/if}
				</div>
			</div>

			<div class="p-1 ">
				<Button on:click={toggleExpanded} color="light" size="xs">
					{#if !expanded}
						<Maximize size="14" />
					{:else}
						<Minimize size="14" />
					{/if}
				</Button>
			</div>

			<div
				class={classNames(
					'text-2xs ml-0.5 font-bold px-2 py-0.5 rounded-sm',
					$selectedComponent === 'ctx' ? 'bg-indigo-500 text-white' : ' bg-indigo-50'
				)}
			>
				ctx
			</div>
			<ComponentOutputViewer
				componentId={'ctx'}
				outputs={['email', 'username', 'query', 'hash']}
				on:select={({ detail }) => {
					$connectingInput = connectInput($connectingInput, 'ctx', detail)
				}}
			/>

			{#each sortGridItemsPosition($app.grid, $breakpoint) as gridItem, index}
				<ComponentOutput {gridItem} first={index === 0} {expanded} />
			{/each}
		</div>
	</div>
</PanelSection>
