<script lang="ts">
	import { classNames } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { flip } from 'svelte/animate'
	import { fade } from 'svelte/transition'
	import type { AppEditorContext } from '../../types'
	import { findGridItem } from '../appUtils'
	import { components } from '../component'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	const { connectingInput, staticOutputs, worldStore, selectedComponent, app } =
		getContext<AppEditorContext>('AppEditorContext')

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
	$: panels = [['ctx', ['email', 'username', 'query']] as [string, string[]]].concat(
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
		<div id="app-tutorial-2" class="relative p-2">
			{#each filteredPanels as [componentId, outputs] (componentId)}
				<div
					animate:flip={{ duration: 300 }}
					in:fade|local={{ duration: 100, delay: 50 }}
					out:fade|local={{ duration: 100 }}
					class="pb-2"
				>
					{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
						{@const name = getComponentNameById(componentId)}
						<div>
							<div
								class="flex {$connectingInput?.opened
									? 'bg-white z-50'
									: ''} flex-row justify-between w-full"
							>
								<button
									on:click|stopPropagation|preventDefault={$connectingInput.opened
										? undefined
										: () => ($selectedComponent = componentId)}
									class={classNames(
										'px-2 text-2xs py-0.5 border-t border-x font-bold rounded-t-sm w-fit',
										$selectedComponent === componentId
											? ' bg-indigo-500/90 text-white border-indigo-500/90'
											: 'bg-gray-100 text-gray-500 border-gray-200'
									)}
								>
									{componentId}
								</button>
								<span
									class={classNames(
										'px-1 text-2xs py-0.5 font-semibold rounded-t-sm w-fit',
										'bg-gray-700 text-white'
									)}
								>
									{name}
								</span>
							</div>
							<div
								class={classNames(
									$connectingInput?.opened ? 'bg-white z-50' : '',
									`w-full py-2 grow border relative break-all `,
									$selectedComponent === componentId ? 'border border-indigo-500/90 ' : '',
									$connectingInput.hoveredComponent === componentId
										? 'outline outline-indigo-500/90'
										: ''
								)}
							>
								{#key $selectedComponent}
									{#key $connectingInput?.opened}
										<ComponentOutputViewer
											outputs={$connectingInput?.opened && $selectedComponent === componentId
												? name == 'Table'
													? ['search']
													: []
												: outputs}
											{componentId}
											on:select={({ detail }) => {
												connectInput(componentId, detail)
											}}
										/>
									{/key}
								{/key}
							</div>
						</div>
					{/if}
				</div>
			{:else}
				<div
					in:fade|local={{ duration: 50, delay: 100 }}
					out:fade|local={{ duration: 50 }}
					class="absolute left-0 top-0 w-full text-sm text-gray-500 text-center py-4 px-2"
				>
					No outputs found
				</div>
			{/each}
		</div>
	</div>
</PanelSection>
