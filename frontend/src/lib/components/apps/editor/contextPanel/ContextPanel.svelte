<script lang="ts">
	import { X } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import type { AppViewerContext } from '../../types'
	import { findGridItem } from '../appUtils'
	import { components } from '../component'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutput from './ComponentOutput.svelte'

	const { connectingInput, staticOutputs, worldStore, selectedComponent, app } =
		getContext<AppViewerContext>('AppViewerContext')

	const manualyOpenedIds = new Set<string>()

	function connectInput(componentId: string, path: string) {
		if ($connectingInput) {
			$connectingInput = {
				opened: false,
				input: {
					connection: {
						componentId,
						path
					},
					type: 'connected'
				},
				hoveredComponent: undefined
			}
		}
	}

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

	$: panels = [['ctx', ['email', 'username', 'query', 'hash']] as [string, string[]]].concat(
		Object.entries($staticOutputs)
	)

	let search = ''

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

		{#each $app.grid as gridItem}
			<ComponentOutput {gridItem} />
		{/each}
	</div>
</PanelSection>
