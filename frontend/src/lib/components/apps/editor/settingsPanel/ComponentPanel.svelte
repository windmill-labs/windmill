<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faChevronDown, faChevronUp, faCopy, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import TableActions from './TableActions.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { capitalize, classNames } from '$lib/utils'
	import { buildExtraLib, fieldTypeToTsType } from '../../utils'
	import Recompute from './Recompute.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ComponentInputTypeEditor from './ComponentInputTypeEditor.svelte'
	import AlignmentEditor from './AlignmentEditor.svelte'
	import RunnableInputEditor from './inputEditor/RunnableInputEditor.svelte'
	import TemplateEditor from '$lib/components/TemplateEditor.svelte'
	import type { AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import GridTab from './GridTab.svelte'
	import { deleteGridItem } from '../appUtils'
	import GridPane from './GridPane.svelte'
	import { slide } from 'svelte/transition'
	import { push } from '$lib/history'
	import Kbd from '$lib/components/common/kbd/Kbd.svelte'

	export let component: AppComponent
	export let rowColumns = false
	export let onDelete: (() => void) | undefined = undefined
	export let parent: string | undefined
	export let noGrid = false
	export let duplicateMoveAllowed = true

	const {
		app,
		staticOutputs,
		runnableComponents,
		selectedComponent,
		worldStore,
		focusedGrid,
		stateId
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		push(history, $app)
		$selectedComponent = undefined
		$focusedGrid = undefined
		if (component && !noGrid) {
			let ids = deleteGridItem($app, component, parent, false)
			for (const key of ids) {
				delete $staticOutputs[key]
				delete $runnableComponents[key]
			}
		}

		delete $staticOutputs[component.id]
		delete $runnableComponents[component.id]
		$app = $app
		$staticOutputs = $staticOutputs
		$runnableComponents = $runnableComponents

		onDelete?.()
	}

	let viewCssOptions = false

	$: extraLib =
		component?.componentInput?.type === 'template' && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, component?.id, false)
			: undefined

	function keydown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Delete': {
				removeGridElement()
				event.stopPropagation()
				break
			}
		}
	}
</script>

<svelte:window on:keydown={keydown} />

{#if component}
	<div class="flex min-h-full flex-col min-w-[150px] w-full divide-y">
		{#if component.componentInput}
			<PanelSection
				smallPadding
				title={component.componentInput.fieldType === 'any' ? 'By Runnable' : 'Input'}
			>
				<svelte:fragment slot="action">
					<span
						class={classNames(
							'text-white px-2 text-2xs py-0.5 font-bold rounded-sm w-fit',
							'bg-indigo-500'
						)}
					>
						{`${component.id}`}
					</span>
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
					{:else if component.componentInput.type === 'template' && component.componentInput !== undefined}
						<div class="py-1 min-h-[28px]  rounded border border-1 border-gray-500">
							<TemplateEditor fontSize={12} bind:code={component.componentInput.eval} {extraLib} />
						</div>
					{:else if component.componentInput.type === 'connected' && component.componentInput !== undefined}
						<ConnectedInputEditor bind:componentInput={component.componentInput} />
					{:else if component.componentInput?.type === 'runnable' && component.componentInput !== undefined}
						<RunnableInputEditor
							bind:appInput={component.componentInput}
							defaultUserInput={component.type == 'formcomponent' ||
								component.type == 'formbuttoncomponent'}
						/>
					{/if}
				</div>
				{#key $stateId}
					{#if component.componentInput?.type === 'runnable'}
						{#if Object.keys(component.componentInput.fields ?? {}).length > 0}
							<div class="border w-full">
								<PanelSection
									smallPadding
									title={`Runnable Inputs (${
										Object.keys(component.componentInput.fields ?? {}).length
									})`}
								>
									<svelte:fragment slot="action">
										<Tooltip>
											The runnable inputs of a button component are not settable by the user. They
											must be defined statically or connected.
										</Tooltip>
									</svelte:fragment>

									<InputsSpecsEditor
										id={component.id}
										shouldCapitalize={false}
										bind:inputSpecs={component.componentInput.fields}
										userInputEnabled={component.type !== 'buttoncomponent'}
										{rowColumns}
									/>
								</PanelSection>
							</div>
						{/if}
					{/if}
				{/key}
			</PanelSection>
		{/if}
		{#if Object.values(component.configuration).length > 0}
			<PanelSection title={`Configuration (${Object.values(component.configuration).length})`}>
				<InputsSpecsEditor
					{rowColumns}
					id={component.id}
					bind:inputSpecs={component.configuration}
					userInputEnabled={false}
				/>
			</PanelSection>
		{/if}

		{#if component.type === 'tabscomponent'}
			<GridTab bind:tabs={component.tabs} {component} />
		{:else if component.type === 'verticalsplitpanescomponent' || component.type === 'horizontalsplitpanescomponent'}
			<GridPane bind:panes={component.panes} {component} />
		{:else if component.type === 'tablecomponent' && Array.isArray(component.actionButtons)}
			<TableActions id={component.id} bind:components={component.actionButtons} />
		{/if}

		<AlignmentEditor bind:component />
		{#if component.type === 'buttoncomponent' || component.type === 'formcomponent' || component.type === 'formbuttoncomponent'}
			<Recompute bind:recomputeIds={component.recomputeIds} ownId={component.id} />
		{/if}

		<div class="grow shrink" />

		{#if Object.keys(component.customCss ?? {}).length > 0}
			<PanelSection title="Custom CSS">
				<div slot="action">
					<Button
						color="light"
						size="xs"
						endIcon={{ icon: viewCssOptions ? faChevronUp : faChevronDown }}
						on:click={() => (viewCssOptions = !viewCssOptions)}
					>
						{viewCssOptions ? 'Hide' : 'Show'}
					</Button>
				</div>
				{#if viewCssOptions}
					<div transition:slide|local>
						{#each Object.keys(component.customCss ?? {}) as name}
							{#if component?.customCss?.[name]}
								<div class="w-full mb-2">
									<CssProperty {name} bind:value={component.customCss[name]} />
								</div>
							{/if}
						{/each}
					</div>
				{/if}
			</PanelSection>
		{/if}

		{#if duplicateMoveAllowed}
			<PanelSection title="Copy/Move">
				<div slot="action">
					<Button size="xs" color="red" variant="border" on:click={removeGridElement}>
						Delete&nbsp;&nbsp;
						<Kbd>Del</Kbd>
					</Button>
				</div>
				<div class="flex flex-col">
					<div
						><span class="text-gray-600 text-xs mr-2">Copy:</span><Kbd>CTRL+C</Kbd> + <Kbd
							>CTRL+V</Kbd
						></div
					>
					<!-- <Button
					size="xs"
					color="dark"
					startIcon={{ icon: faCopy }}
					on:click={() => {
						if (component) {
							duplicateElement(component.id)
						}
					}}
				>
					Duplicate component: {component.id}
				</Button> -->
					<div
						><span class="text-gray-600 text-xs mr-2">Move:</span><Kbd>CTRL+X</Kbd> + <Kbd
							>CTRL+V</Kbd
						></div
					>
					<div
						><span class="text-gray-600 text-xs mr-2">Navigate:</span><Kbd>&leftarrow;</Kbd><Kbd
							>&uparrow;</Kbd
						><Kbd>&rightarrow;</Kbd><Kbd>ESC</Kbd></div
					>
				</div>
			</PanelSection>
		{/if}
	</div>
{/if}
