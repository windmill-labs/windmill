<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import PickScript from '$lib/components/flows/pickers/PickScript.svelte'
	import {
		faAlignCenter,
		faAlignLeft,
		faAlignRight,
		faClose,
		faPen,
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
	import PickInlineScript from './PickInlineScript.svelte'

	export let component: AppComponent | undefined

	const { app, staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		if (component) {
			const COLS = 6
			$app.grid = $app.grid.filter((gridComponent) => gridComponent.data.id !== component?.id)
			$app.grid = gridHelp.adjust($app.grid, COLS)

			// Delete static inputs
			delete $staticOutputs[component.id]
			$staticOutputs = $staticOutputs
		}
	}
</script>

{#if component}
	<div class="flex flex-col w-full divide-y">
		{#if component.runnable}
			<PanelSection title="Runnable">
				{#if component['inlineScriptName']}
					<span class="text-xs">{component['inlineScriptName']}</span>
					<div class="w-full flex flex-row gap-2">
						<Button
							size="xs"
							color="dark"
							startIcon={{ icon: faPen }}
							on:click={() => {
								alert('TODO')
							}}
						>
							Edit
						</Button>

						<Button
							size="xs"
							color="light"
							variant="border"
							startIcon={{ icon: faClose }}
							on:click={() => {
								alert('TODO')
							}}
						>
							Clear
						</Button>
					</div>
				{/if}

				{#if component.runnable && component['path'] === undefined && component['inlineScriptName'] === undefined}
					<span class="text-sm">Select a script or a flow to continue</span>
					<PickInlineScript
						scripts={(Object.keys($app.inlineScripts) || []).map((summary) => ({summary}))}
						on:pick={({ detail }) => {
							if (component?.runnable) {
								// @ts-ignore
								component.inlineScriptName = detail.summary
							}
						}}
					/>
					<PickScript
						kind="script"
						on:pick={({ detail }) => {
							if (component?.runnable) {
								component['path'] = detail.path
								component['runType'] = 'script'
							}
						}}
					/>
					<PickFlow
						on:pick={({ detail }) => {
							if (component?.runnable) {
								component['path'] = detail.path
								component['runType'] = 'flow'
							}
						}}
					/>
				{/if}

				<!-- {#if component.runnable && component['path'] === undefined && component['inlineScriptName'] === undefined}
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
				{/if} -->
			</PanelSection>
		{/if}
		{#if Object.values(component.inputs).length > 0}
			<PanelSection title="Runnable inputs">
				<InputsSpecsEditor bind:inputSpecs={component.inputs} />
			</PanelSection>
		{/if}

		{#if Object.values(component.componentInputs).length > 0}
			<PanelSection title="Component parameters">
				<ComponentInputsSpecsEditor bind:componentInputSpecs={component.componentInputs} />
			</PanelSection>
		{/if}

		{#if component.verticalAlignment !== undefined}
			<PanelSection title="Alignment">
				<div class="w-full text-xs font-bold">Horizontal alignment</div>

				<ToggleButtonGroup bind:selected={component.horizontalAlignment}>
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
				<div class="w-full text-xs font-bold">Vertical alignment</div>

				<ToggleButtonGroup bind:selected={component.verticalAlignment}>
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
