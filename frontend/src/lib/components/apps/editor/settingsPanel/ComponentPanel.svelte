<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import PickScript from '$lib/components/flows/pickers/PickScript.svelte'
	import {
		faAlignCenter,
		faAlignLeft,
		faAlignRight,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { AppComponent, AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import ComponentInputsSpecsEditor from './ComponentInputsSpecsEditor.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import PickFlow from './PickFlow.svelte'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'

	export let component: AppComponent | undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		const COLS = 6
		const index = $app.grid.findIndex((gridComponent) => gridComponent.data.id === component?.id)
		$app.grid.splice(index, 1)
		$app.grid = gridHelp.adjust($app.grid, COLS)
	}
</script>

{#if component}
	<div class="flex flex-col w-full divide-y">
		<PanelSection title="Context">
			{#if component.inputs}
				<InputsSpecsEditor bind:inputSpecs={component.inputs} />
			{/if}

			{#if component.runnable}
				<span class="text-sm">Select a script or a flow to continue</span>
				<PickScript
					kind="script"
					on:pick={({ detail }) => {
						if (component && component.type === 'runformcomponent') {
							component.path = detail.path
							component.runType = 'script'
						}
					}}
				/>
				<PickFlow
					on:pick={({ detail }) => {
						if (component && component.type === 'runformcomponent') {
							component.path = detail.path
							component.runType = 'flow'
						}
					}}
				/>
			{/if}

			{#if component.runnable}
				{#each Object.keys($app.inlineScripts ?? {}) as inlineScriptName}
					<Button
						on:click={() => {
							if (component?.runnable) {
								// @ts-ignore
								component.inlineScriptName = inlineScriptName
							}
						}}
						size="xs"
					>
						Link {inlineScriptName}
					</Button>
				{/each}
			{/if}

			{#if component.componentInputs}
				<ComponentInputsSpecsEditor bind:componentInputSpecs={component.componentInputs} />
			{/if}
		</PanelSection>

		{#if component.verticalAlignement !== undefined}
			<PanelSection title="Alignement">
				<div class="w-full text-xs font-bold">Horizontal alignement</div>

				<ToggleButtonGroup bind:selected={component.horizontalAlignement}>
					<ToggleButton position="left" value="left" size="xs">
						<Icon data={faAlignLeft} />
					</ToggleButton>
					<ToggleButton position="center" value="center" size="xs">
						<Icon data={faAlignCenter} />
					</ToggleButton>
					<ToggleButton position="right" value="right" size="xs">
						<Icon data={faAlignRight} />
					</ToggleButton>
				</ToggleButtonGroup>
				<div class="w-full text-xs font-bold">Vertical alignement</div>

				<ToggleButtonGroup bind:selected={component.verticalAlignement}>
					<ToggleButton position="left" value="top" size="xs">
						<Icon data={faAlignLeft} />
					</ToggleButton>
					<ToggleButton position="center" value="center" size="xs">
						<Icon data={faAlignCenter} />
					</ToggleButton>
					<ToggleButton position="right" value="bottom" size="xs">
						<Icon data={faAlignRight} />
					</ToggleButton>
				</ToggleButtonGroup>
			</PanelSection>
		{/if}
		<PanelSection title="Danger zone">
			<Button
				size="xs"
				color="red"
				variant="border"
				startIcon={{ icon: faTrashAlt }}
				on:click={removeGridElement}
			>
				Delete component
			</Button>
		</PanelSection>
	</div>
{/if}
