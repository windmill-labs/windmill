<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import PickScript from '$lib/components/flows/pickers/PickScript.svelte'
	import {
		faAlignCenter,
		faAlignLeft,
		faAlignRight,
		faClose,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { AppComponent, AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import PickFlow from './PickFlow.svelte'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import PickInlineScript from './PickInlineScript.svelte'
	import TableActions from './TableActions.svelte'
	import { capitalize } from '$lib/utils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { gridColumns } from '../../gridUtils'
	import { Plus } from 'svelte-lucide'

	export let component: AppComponent | undefined
	export let onDelete: (() => void) | undefined = undefined

	const { app, staticOutputs } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		if (onDelete && component) {
			delete $staticOutputs[component.id]
			$staticOutputs = $staticOutputs

			onDelete()
			// Delete static inputs
		} else {
			if (component) {
				$app.grid = $app.grid.filter((gridComponent) => gridComponent.data.id !== component?.id)

				gridColumns.forEach((colIndex) => {
					$app.grid = gridHelp.adjust($app.grid, colIndex)
				})

				// Delete static inputs
				delete $staticOutputs[component.id]
				$staticOutputs = $staticOutputs
			}
		}
	}
</script>

{#if component}
	<div class="flex flex-col w-full divide-y">
		{#if component.runnable}
			<PanelSection title="Runnable">
				{#if component.runnable && component['inlineScriptName']}
					<div class="flex justify-between w-full items-center">
						<span class="text-xs">{component['inlineScriptName']}</span>
						<Button
							size="xs"
							color="red"
							startIcon={{ icon: faClose }}
							on:click={() => {
								if (component) {
									component['inlineScriptName'] = undefined
								}
							}}
						>
							Clear
						</Button>
					</div>
				{/if}

				{#if component.runnable && component['path']}
					<div class="flex gap-2 items-center">
						<div>
							<Badge color="blue">{capitalize(component['runType'])}</Badge>
							<span class="text-xs">{component['path']}</span>
						</div>
						<Button
							size="xs"
							color="red"
							variant="border"
							startIcon={{ icon: faClose }}
							on:click={() => {
								if (component) {
									component['path'] = undefined
								}
							}}
						>
							Clear
						</Button>
					</div>
				{/if}

				{#if component.runnable && component['path'] === undefined && component['inlineScriptName'] === undefined}
					<div class="text-sm">Inline scripts:</div>
					<div class="flex gap-2">
						<Button
							btnClasses="w-24 truncate"
							size="sm"
							spacingSize="md"
							variant="border"
							color="light"
						>
							<div class="flex justify-center flex-col items-center gap-2">
								<Plus size="18px" />

								<span class="text-xs">Create</span>
							</div>
						</Button>

						<PickInlineScript
							scripts={(Object.keys($app.inlineScripts) || []).map((summary) => ({ summary }))}
							on:pick={({ detail }) => {
								if (component?.runnable) {
									// @ts-ignore
									component.inlineScriptName = detail.summary
								}
							}}
						/>
					</div>

					<div class="text-sm">Pick from workspace:</div>
					<div class="flex gap-2">
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
					</div>
				{/if}
			</PanelSection>
		{/if}
		{#if Object.values(component.inputs).length > 0}
			<PanelSection title="Runnable inputs">
				<InputsSpecsEditor bind:inputSpecs={component.inputs} />
			</PanelSection>
		{/if}

		{#if Object.values(component.componentInputs).length > 0}
			<PanelSection
				title={`Component parameters (${Object.values(component.componentInputs).length})`}
			>
				<InputsSpecsEditor bind:inputSpecs={component.componentInputs} userInputEnabled={false} />
			</PanelSection>
		{/if}

		{#if component.type === 'tablecomponent' && Array.isArray(component.actionButtons)}
			<TableActions bind:components={component.actionButtons} />
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
				Delete component: {component.id}
			</Button>
		</PanelSection>
	</div>
{/if}
