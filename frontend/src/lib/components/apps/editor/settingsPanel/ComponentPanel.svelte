<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		faArrowRight,
		faClose,
		faCode,
		faMinimize,
		faPen,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import {
		AlignStartHorizontal,
		AlignStartVertical,
		AlignCenterHorizontal,
		AlignCenterVertical,
		AlignEndHorizontal,
		AlignEndVertical
	} from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import TableActions from './TableActions.svelte'
	import StaticInputEditor from './StaticInputEditor.svelte'
	import ConnectedInputEditor from './ConnectedInputEditor.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { capitalize } from '$lib/utils'
	import { fieldTypeToTsType } from '../../utils'
	import Recompute from './Recompute.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import RunnableSelector from './mainInput/RunnableSelector.svelte'
	import gridHelp from 'svelte-grid/build/helper/index.mjs'
	import { gridColumns } from '../../gridUtils'

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
				{#if component.componentInput.fieldType !== 'any'}
					<div class="w-full">
						<ToggleButtonGroup bind:selected={component.componentInput.type}>
							<ToggleButton
								position="left"
								value="static"
								startIcon={{ icon: faPen }}
								size="xs"
								disable={component.type === 'buttoncomponent'}
							>
								Static
							</ToggleButton>
							<ToggleButton
								value="connected"
								position="center"
								startIcon={{ icon: faArrowRight }}
								size="xs"
							>
								Connected
							</ToggleButton>
							<ToggleButton
								position="right"
								value="runnable"
								startIcon={{ icon: faCode }}
								size="xs"
							>
								Computed
							</ToggleButton>
						</ToggleButtonGroup>
					</div>
				{/if}
				<div class="flex flex-col w-full gap-2 my-2">
					{#if component.componentInput.type === 'static'}
						<StaticInputEditor bind:componentInput={component.componentInput} />
					{:else if component.componentInput.type === 'connected' && component.componentInput !== undefined}
						<ConnectedInputEditor bind:componentInput={component.componentInput} />
					{:else if component && component.componentInput?.type === 'runnable' && component.componentInput.runnable}
						<div class="flex justify-between w-full items-center">
							<span class="text-xs">
								{component.componentInput.runnable.type === 'runnableByName'
									? component.componentInput.runnable.inlineScriptName
									: component.componentInput.runnable.path}
							</span>
							<Button
								size="xs"
								color="red"
								variant="border"
								startIcon={{ icon: faClose }}
								on:click={() => {
									if (component?.componentInput?.type === 'runnable') {
										component.componentInput.runnable = undefined
										component.componentInput.fields = {}
										component = component
									}
								}}
							>
								Clear
							</Button>
						</div>
					{:else}
						<RunnableSelector
							inlineScripts={Object.keys($app.inlineScripts)}
							bind:componentInput={component.componentInput}
						/>
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
							{#if component.type === 'buttoncomponent'}
								<Alert title="Button inputs" type="info" size="xs">
									The runnable inputs of a button component are not settable by the user. They must
									be defined statically or connected.
								</Alert>
							{/if}

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

		{#if component.verticalAlignment !== undefined}
			<PanelSection title="Alignment">
				<svelte:fragment slot="action">
					<Button
						size="xs"
						on:click={() => {
							if (component) {
								component.verticalAlignment = 'center'
								component.horizontalAlignment = 'center'
							}
						}}
						startIcon={{ icon: faMinimize }}
					>
						Center
					</Button>
				</svelte:fragment>
				<div class="w-full text-xs font-semibold">Horizontal alignment</div>

				<div class="w-full">
					<ToggleButtonGroup bind:selected={component.horizontalAlignment}>
						<ToggleButton position="left" value="left" size="xs">
							<AlignStartVertical size={16} />
						</ToggleButton>
						<ToggleButton position="center" value="center" size="xs">
							<AlignCenterVertical size={16} />
						</ToggleButton>
						<ToggleButton position="right" value="right" size="xs">
							<AlignEndVertical size={16} />
						</ToggleButton>
					</ToggleButtonGroup>
				</div>
				{#if component.type !== 'formcomponent'}
					<div class="w-full text-xs font-semibold">Vertical alignment</div>
					<div class="w-full">
						<ToggleButtonGroup bind:selected={component.verticalAlignment}>
							<ToggleButton position="left" value="top" size="xs">
								<AlignStartHorizontal size={16} />
							</ToggleButton>
							<ToggleButton position="center" value="center" size="xs">
								<AlignCenterHorizontal size={16} />
							</ToggleButton>
							<ToggleButton position="right" value="bottom" size="xs">
								<AlignEndHorizontal size={16} />
							</ToggleButton>
						</ToggleButtonGroup>
					</div>
				{/if}
			</PanelSection>
		{/if}
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
