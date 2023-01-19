<script lang="ts">
	import { page } from '$app/stores'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import { key } from 'svelte-awesome/icons'
	import type { AppEditorContext } from '../../types'
	import { displayData } from '../../utils'
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
		const component = $app.grid.find((c) => c?.data?.id === componentId)

		if (component?.data.type) {
			return displayData[component?.data.type].name
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
</script>

<PanelSection noPadding titlePadding="px-4 pt-2" title="Outputs">
	<div
		class="overflow-auto min-w-[150px] border-t w-full relative flex flex-col gap-4 px-2 pt-4 pb-2"
	>
		{#each panels as [componentId, outputs] (componentId)}
			{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
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
								'px-2 text-2xs py-0.5 border border-gray-300 font-bold rounded-t-sm w-fit',
								$selectedComponent === componentId
									? ' bg-indigo-500 text-white'
									: 'bg-gray-200 text-gray-500'
							)}
						>
							{componentId}
						</button>
						<span
							class={classNames(
								'px-1 text-2xs py-0.5 font-bold rounded-t-sm w-fit',
								'bg-gray-500 text-white'
							)}
						>
							{getComponentNameById(componentId)}
						</span>
					</div>

					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						class={classNames(
							$connectingInput?.opened ? 'bg-white z-50' : '',
							`w-full py-2 grow border relative break-all `,
							$selectedComponent === componentId ? 'border border-blue-500 ' : '',
							$connectingInput.hoveredComponent === componentId ? 'outline outline-blue-500' : ''
						)}
					>
						{#key $selectedComponent}
							{#key $connectingInput?.opened}
								<ComponentOutputViewer
									outputs={$connectingInput?.opened && $selectedComponent === componentId
										? ['search']
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
		{/each}
	</div>
</PanelSection>
