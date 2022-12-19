<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import TableActions from './TableActions.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { capitalize } from '$lib/utils'
	import { fieldTypeToTsType } from '../../utils'
	import Recompute from './Recompute.svelte'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { gridColumns } from '../../gridUtils'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import SelectedRunnable from './SelectedRunnable.svelte'
	import ComponentInputTypeEditor from './ComponentInputTypeEditor.svelte'
	import AlignmentEditor from './AlignmentEditor.svelte'
	import RunnableInputEditor from './inputEditor/RunnableInputEditor.svelte'

	export let component: AppComponent | undefined
	export let onDelete: (() => void) | undefined = undefined

	const { app, staticOutputs, runnableComponents } =
		getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		if (onDelete && component) {
			delete $staticOutputs[component.id]
			$staticOutputs = $staticOutputs

			delete $runnableComponents[component.id]
			$runnableComponents = $runnableComponents

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

				delete $runnableComponents[component.id]
				$runnableComponents = $runnableComponents
			}
		}
	}
</script>

{#if component}
	<div class="flex flex-col w-full divide-y">
		{#if component.componentInput}
			<PanelSection
				title={component.componentInput.fieldType === 'any' ? 'Runnable' : 'Component input'}
			>
				<svelte:fragment slot="action">
					{#if component.componentInput.fieldType !== 'any'}
						<Badge color="blue">
							{component.componentInput.fieldType === 'array' &&
							component.componentInput.subFieldType
								? `${capitalize(fieldTypeToTsType(component.componentInput.subFieldType))}[]`
								: capitalize(fieldTypeToTsType(component.componentInput.fieldType))}
						</Badge>
					{/if}
				</svelte:fragment>

				<ComponentInputTypeEditor bind:componentInput={component.componentInput} />
				<div class="flex flex-col w-full gap-2 my-2">
					{#if component.componentInput.type === 'static'}
						<StaticInputEditor bind:componentInput={component.componentInput} />
					{:else if component.componentInput.type === 'connected' && component.componentInput !== undefined}
						<ConnectedInputEditor bind:componentInput={component.componentInput} />
					{:else if component.componentInput?.type === 'runnable' && component.componentInput !== undefined}
						<RunnableInputEditor bind:appInput={component.componentInput} />
					{/if}
				</div>
				{#if component.componentInput?.type === 'runnable' && Object.keys(component.componentInput.fields ?? {}).length > 0}
					<div class="border w-full">
						<PanelSection
							smallPadding
							title={`Runnable inputs (${
								Object.keys(component.componentInput.fields ?? {}).length
							})`}
						>
							<svelte:fragment slot="action">
								<Tooltip>
									The runnable inputs of a button component are not settable by the user. They must
									be defined statically or connected.
								</Tooltip>
							</svelte:fragment>

							<InputsSpecsEditor
								bind:inputSpecs={component.componentInput.fields}
								userInputEnabled={component.type !== 'buttoncomponent'}
							/>
						</PanelSection>
					</div>
				{/if}
			</PanelSection>
		{/if}

		{#if Object.values(component.configuration).length > 0}
			<PanelSection title={`Configuration (${Object.values(component.configuration).length})`}>
				<InputsSpecsEditor bind:inputSpecs={component.configuration} userInputEnabled={false} />
			</PanelSection>
		{/if}

		{#if component.type === 'tablecomponent' && Array.isArray(component.actionButtons)}
			<TableActions bind:components={component.actionButtons} />
		{/if}

		<AlignmentEditor bind:component />
		{#if component.type === 'buttoncomponent' || component.type === 'formcomponent'}
			<Recompute bind:recomputeIds={component.recomputeIds} ownId={component.id} />
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
