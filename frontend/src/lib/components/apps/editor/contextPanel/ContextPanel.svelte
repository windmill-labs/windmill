<script lang="ts">
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import PanelSection from '../settingsPanel/common/PanelSection.svelte'
	import ComponentOutputViewer from './ComponentOutputViewer.svelte'

	const { connectingInput, staticOutputs, worldStore, selectedComponent } =
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
</script>

<PanelSection title="Outputs">
	{#each Object.entries($staticOutputs) as [componentId, outputs], index}
		{#if outputs.length > 0 && $worldStore?.outputsById[componentId]}
			<button
				on:click={() => ($selectedComponent = componentId)}
				class={classNames(
					'text-xs px-2 py-1 -mb-2 rounded-t-',
					$selectedComponent === componentId
						? 'bg-blue-500 text-white'
						: 'bg-gray-200 text-gray-500'
				)}
			>
				Component: {componentId}
			</button>

			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				on:click={() => ($selectedComponent = componentId)}
				class={classNames(
					'w-full py-2 border',
					$selectedComponent === componentId ? 'border border-blue-500' : 'cursor-pointer'
				)}
			>
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
