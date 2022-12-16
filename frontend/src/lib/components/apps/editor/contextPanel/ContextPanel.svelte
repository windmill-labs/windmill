<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
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
			<Button size="xs" on:click={() => ($selectedComponent = componentId)} color="blue">
				Component: {componentId}
			</Button>

			<div
				class={classNames(
					'w-full p-2 rounded-xs border',
					$selectedComponent === componentId
						? 'outline-1 outline outline-offset-2 outline-blue-500'
						: ''
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
