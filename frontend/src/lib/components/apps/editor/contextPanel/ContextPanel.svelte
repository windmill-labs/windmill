<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
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
				}
			}
		}
	}

	function getComponentNameById(componentId: string) {
		const component = $app.grid.find((c) => c.data.id === componentId)

		if (component?.data.type) {
			return displayData[component?.data.type].name
		}
	}
</script>

<PanelSection title="Outputs">
	{#each Object.entries($staticOutputs) as [componentId, outputs] (componentId)}
		{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
			<div class="flex flex-row justify-between w-full -mb-2 ">
				<button
					on:click={() => ($selectedComponent = componentId)}
					class={classNames(
						'px-2 text-2xs py-0.5 font-bold rounded-t-sm w-fit',
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
				on:click={() => ($selectedComponent = componentId)}
				class={classNames(
					'w-full py-2 border relative',
					$selectedComponent === componentId ? 'border border-blue-500 ' : 'cursor-pointer'
				)}
			>
				{#if $selectedComponent === componentId && $connectingInput?.opened}
					<div class="absolute top-0 left-0 w-full h-full bg-gray-500 bg-opacity-50 z-10" />
				{/if}
				<ComponentOutputViewer
					{outputs}
					{componentId}
					on:select={({ detail }) => {
						connectInput(componentId, detail)
					}}
				/>
			</div>
		{/if}
	{/each}
</PanelSection>
