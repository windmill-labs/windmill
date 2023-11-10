<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext, GridItem, RichConfiguration } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import TableActions from './TableActions.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import { classNames, getModifierKey, isMac } from '$lib/utils'
	import { buildExtraLib } from '../../utils'
	import Recompute from './Recompute.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ComponentInputTypeEditor from './ComponentInputTypeEditor.svelte'
	import AlignmentEditor from './AlignmentEditor.svelte'
	import RunnableInputEditor from './inputEditor/RunnableInputEditor.svelte'
	import TemplateEditor from '$lib/components/TemplateEditor.svelte'
	import { ccomponents, components } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import GridTab from './GridTab.svelte'
	import { deleteGridItem } from '../appUtils'
	import GridPane from './GridPane.svelte'
	import { slide } from 'svelte/transition'
	import { push } from '$lib/history'
	import Kbd from '$lib/components/common/kbd/Kbd.svelte'
	import StylePanel from './StylePanel.svelte'
	import { ChevronLeft, Delete, ExternalLink } from 'lucide-svelte'
	import GridCondition from './GridCondition.svelte'
	import { isTriggerable } from './script/utils'
	import { inferDeps } from '../appUtilsInfer'
	import EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'
	import type { ResultAppInput } from '../../inputType'
	import GridGroup from './GridGroup.svelte'
	import { secondaryMenuLeft } from './secondaryMenu'

	import ComponentControl from './ComponentControl.svelte'
	import GridAgGridLicenseKey from './GridAgGridLicenseKey.svelte'
	import ComponentPanelDataSource from './ComponentPanelDataSource.svelte'

	export let componentSettings: { item: GridItem; parent: string | undefined } | undefined =
		undefined
	export let onDelete: (() => void) | undefined = undefined
	export let noGrid = false
	export let duplicateMoveAllowed = true

	const {
		app,
		runnableComponents,
		selectedComponent,
		worldStore,
		focusedGrid,
		stateId,
		state,
		errorByComponent,
		componentControl
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history, movingcomponents } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		push(history, $app)

		const id = componentSettings?.item?.id
		const onDeleteComponentControl = id ? $componentControl[id]?.onDelete : undefined
		if (onDeleteComponentControl) {
			onDeleteComponentControl()
		}
		if (onDelete) {
			onDelete()
		}

		let cId = componentSettings?.item.id
		if (cId) {
			delete $worldStore.outputsById[cId]
			delete $errorByComponent[cId]

			if ($movingcomponents?.includes(cId)) {
				$movingcomponents = $movingcomponents.filter((id) => id !== cId)
			}
		}

		$selectedComponent = undefined
		$focusedGrid = undefined
		if (componentSettings?.item && !noGrid) {
			let ids = deleteGridItem($app, componentSettings?.item.data, componentSettings?.parent)
			for (const key of ids) {
				delete $runnableComponents[key]
			}
		}

		if (componentSettings?.item?.data?.id) {
			delete $runnableComponents[componentSettings?.item?.data?.id]
		}
		$app = $app
		$runnableComponents = $runnableComponents

		onDelete?.()
	}

	let viewCssOptions = false

	$: extraLib =
		(componentSettings?.item?.data?.componentInput?.type === 'template' ||
			componentSettings?.item?.data?.componentInput?.type === 'templatev2') &&
		$worldStore
			? buildExtraLib(
					$worldStore?.outputsById ?? {},
					componentSettings?.item?.data?.id,
					$state,
					false
			  )
			: undefined

	function keydown(event: KeyboardEvent) {
		const { key, metaKey } = event
		if (key === 'Delete' || (key === 'Backspace' && metaKey)) {
			removeGridElement()
			event.stopPropagation()
		}
	}

	const initialConfiguration = componentSettings?.item?.data?.type
		? ccomponents[componentSettings.item.data.type].initialData.configuration
		: {}

	const componentInput: RichConfiguration | undefined = componentSettings?.item?.data?.type
		? ccomponents[componentSettings?.item?.data?.type].initialData.componentInput
		: undefined

	$: componentSettings?.item?.data && ($app = $app)

	const hasInteraction = componentSettings?.item.data.type
		? isTriggerable(componentSettings?.item.data.type)
		: false

	let evalV2editor: EvalV2InputEditor | undefined = undefined

	function transformToFrontend() {
		if (componentSettings?.item.data.componentInput) {
			const id = componentSettings?.item?.data?.id
			let appInput: ResultAppInput = {
				...componentSettings.item.data.componentInput,
				type: 'runnable',
				runnable: {
					type: 'runnableByName',
					name: `Eval of ${id}`,
					inlineScript: {
						path: `${id}_eval`,
						content: `return ${componentSettings?.item.data.componentInput?.['expr']}`,
						language: 'frontend',
						refreshOn: componentSettings?.item.data.componentInput?.['connections']?.map((c) => {
							return {
								id: c.componentId,
								key: c.id
							}
						})
					}
				},
				fields: {}
			}
			componentSettings.item.data.componentInput = appInput
		}
	}
</script>

<svelte:window on:keydown={keydown} />

{#if componentSettings?.item?.data}
	{@const component = componentSettings.item.data}
	<div class="flex justify-between items-center px-3 py-2 bg-surface-selected">
		<div class="text-xs text-primary font-semibold"
			>{components[componentSettings.item.data.type].name}</div
		>
		<a
			href={components[componentSettings.item.data.type].documentationLink}
			target="_blank"
			class="text-frost-500 dark:text-frost-300 font-semibold text-xs"
		>
			<div class="flex flex-row gap-2">
				See documentation
				<ExternalLink size="16" />
			</div>
		</a>
	</div>

	<div class="flex min-h-[calc(100%-32px)] flex-col min-w-[150px] w-full divide-y">
		<ComponentPanelDataSource bind:component={componentSettings.item.data}>
			{#if component.componentInput}
				<PanelSection
					title={componentSettings?.item.data.type == 'steppercomponent'
						? 'Validations'
						: hasInteraction
						? 'Event handler'
						: 'Data source'}
					id={'component-input'}
				>
					<svelte:fragment slot="action">
						{#if componentSettings.item.data.type !== 'plotlycomponentv2'}
							<span
								class={classNames(
									'text-white px-2 text-2xs py-0.5 font-bold rounded-sm w-fit',
									'bg-indigo-500'
								)}
							>
								{`${component.id}`}
							</span>
						{/if}
					</svelte:fragment>

					{#if componentSettings.item.data.componentInput}
						<ComponentInputTypeEditor
							{evalV2editor}
							bind:componentInput={componentSettings.item.data.componentInput}
							id={component.id}
						/>

						<div class="flex flex-col w-full gap-2 mt-2">
							{#if componentSettings.item.data.componentInput.type === 'static'}
								<StaticInputEditor
									fieldType={componentInput?.fieldType}
									subFieldType={componentInput?.subFieldType}
									format={componentInput?.format}
									bind:componentInput={componentSettings.item.data.componentInput}
								/>
							{:else if componentSettings.item.data?.componentInput?.type === 'template' || componentSettings.item.data?.componentInput?.type === 'templatev2'}
								<div class="py-1 min-h-[28px] rounded border border-1 border-gray-500">
									<TemplateEditor
										fontSize={12}
										bind:code={componentSettings.item.data.componentInput.eval}
										{extraLib}
										on:change={(e) => {
											if (componentSettings?.item.data.componentInput?.type === 'templatev2') {
												inferDeps(
													'`' + e.detail.code + '`',
													$worldStore.outputsById,
													componentSettings?.item.data.componentInput,
													app
												)
											}
										}}
									/>
								</div>
								{#if componentSettings.item.data?.componentInput?.type === 'templatev2'}
									{#if componentSettings.item.data?.componentInput.connections?.length > 0}
										<div class="flex flex-wrap gap-2 items-center">
											<div class="text-2xs text-tertiary">Re-evaluated on changes to:</div>
											<div class="flex flex-wrap gap-1">
												{#each componentSettings.item.data?.componentInput.connections as connection (connection.componentId + '-' + connection.id)}
													<span
														class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border"
													>
														{connection.componentId + '.' + connection.id}
													</span>
												{/each}
											</div>
										</div>
									{/if}
								{/if}
							{:else if componentSettings.item.data.componentInput.type === 'connected' && component.componentInput !== undefined}
								<ConnectedInputEditor
									bind:componentInput={componentSettings.item.data.componentInput}
								/>
							{:else if componentSettings.item.data.componentInput.type === 'evalv2' && component.componentInput !== undefined}
								<EvalV2InputEditor
									bind:this={evalV2editor}
									id={component.id}
									bind:componentInput={componentSettings.item.data.componentInput}
								/>
								<a class="text-2xs" on:click={transformToFrontend} href="#">
									transform to a frontend script
								</a>
							{:else if componentSettings.item.data.componentInput?.type === 'runnable' && component.componentInput !== undefined}
								<RunnableInputEditor
									appComponent={component}
									bind:appInput={componentSettings.item.data.componentInput}
									defaultUserInput={component.type == 'formcomponent' ||
										component.type == 'formbuttoncomponent'}
								/>
							{/if}
						</div>
					{/if}
					{#key $stateId}
						{#if componentSettings.item.data.componentInput?.type === 'runnable'}
							{#if Object.keys(componentSettings.item.data.componentInput.fields ?? {}).length > 0}
								<div class="w-full">
									<div class="flex flex-row items-center gap-2 text-sm font-semibold">
										Runnable Inputs

										<Tooltip wrapperClass="flex">
											The runnable inputs are inferred from the inputs of the flow or script
											parameters this component is attached to.
										</Tooltip>
									</div>

									<InputsSpecsEditor
										id={component.id}
										shouldCapitalize={false}
										displayType
										bind:inputSpecs={componentSettings.item.data.componentInput.fields}
										userInputEnabled={component.type === 'formcomponent' ||
											component.type === 'formbuttoncomponent'}
									/>
								</div>
							{/if}
						{/if}
					{/key}
				</PanelSection>
			{/if}
		</ComponentPanelDataSource>

		<ComponentControl type={component.type} />

		{#if componentSettings.item.data.type === 'tabscomponent'}
			<GridTab
				bind:tabs={componentSettings.item.data.tabs}
				bind:disabledTabs={componentSettings.item.data.disabledTabs}
				bind:component={componentSettings.item.data}
				canDisableTabs
			/>
		{:else if componentSettings.item.data.type === 'aggridcomponentee'}
			<GridAgGridLicenseKey bind:license={componentSettings.item.data.license} />
		{:else if componentSettings.item.data.type === 'steppercomponent'}
			<GridTab
				bind:tabs={componentSettings.item.data.tabs}
				bind:component={componentSettings.item.data}
				word="Step"
			/>
		{:else if componentSettings.item.data.type === 'containercomponent'}
			<GridGroup
				bind:groupFields={componentSettings.item.data.groupFields}
				item={componentSettings.item}
			/>
		{:else if componentSettings.item.data.type === 'conditionalwrapper'}
			<GridCondition
				bind:conditions={componentSettings.item.data.conditions}
				bind:component={componentSettings.item.data}
			/>
		{:else if componentSettings.item.data.type === 'verticalsplitpanescomponent' || componentSettings.item.data.type === 'horizontalsplitpanescomponent'}
			<GridPane
				bind:panes={componentSettings.item.data.panes}
				bind:component={componentSettings.item.data}
			/>
		{:else if componentSettings.item.data.type === 'tablecomponent' && Array.isArray(componentSettings.item.data.actionButtons)}
			<TableActions id={component.id} bind:components={componentSettings.item.data.actionButtons} />
		{/if}

		{#if Object.values(initialConfiguration).length > 0}
			<PanelSection title="Configuration">
				<InputsSpecsEditor
					id={component.id}
					inputSpecsConfiguration={initialConfiguration}
					bind:inputSpecs={componentSettings.item.data.configuration}
					userInputEnabled={false}
				/>
			</PanelSection>
		{:else if componentSettings.item.data.type != 'containercomponent'}
			<div class="h-full w-full text-sm text-tertiary text-center py-8 px-2">
				{ccomponents[component.type].name} has no configuration
			</div>
		{/if}

		{#if (`recomputeIds` in componentSettings.item.data && Array.isArray(componentSettings.item.data.recomputeIds)) || componentSettings.item.data.type === 'buttoncomponent' || componentSettings.item.data.type === 'formcomponent' || componentSettings.item.data.type === 'formbuttoncomponent' || componentSettings.item.data.type === 'checkboxcomponent'}
			<Recompute
				bind:recomputeIds={componentSettings.item.data.recomputeIds}
				ownId={component.id}
			/>
		{/if}

		<div class="grow shrink" />

		{#if Object.keys(ccomponents[component.type].customCss ?? {}).length > 0}
			<PanelSection title="Styling">
				<div slot="action" class="flex justify-end flex-wrap gap-1">
					<Button
						color="light"
						size="xs"
						variant="border"
						startIcon={{ icon: ChevronLeft }}
						on:click={() => secondaryMenuLeft.toggle(StylePanel, {})}
					>
						Show
					</Button>
				</div>
				<AlignmentEditor bind:component={componentSettings.item.data} />
				{#if viewCssOptions}
					<div transition:slide|local class="w-full">
						{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
							{#if componentSettings.item.data?.customCss != undefined}
								<div class="w-full mb-2">
									<CssProperty
										forceStyle={ccomponents[component.type].customCss[name].style != undefined}
										forceClass={ccomponents[component.type].customCss[name].class != undefined}
										tooltip={ccomponents[component.type].customCss[name].tooltip}
										{name}
										bind:value={componentSettings.item.data.customCss[name]}
									/>
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
						{#if isMac()}
							<Kbd kbdClass="center-center">
								<span class="text-lg leading-none">⌘</span>
								<span class="px-0.5">+</span>
								<Delete size={16} />
							</Kbd>
						{:else}
							<Kbd>Del</Kbd>
						{/if}
					</Button>
				</div>
				<div class="flex flex-col gap-1">
					<div>
						<span class="text-secondary text-xs mr-2"> Copy:</span>
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>C</Kbd>,
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>V</Kbd>
					</div>
					<div>
						<span class="text-secondary text-xs mr-2">Move: </span>
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>X</Kbd>,
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>V</Kbd>
					</div>
					<div>
						<span class="text-secondary text-xs mr-2">Navigate:</span>
						<Kbd>&leftarrow;</Kbd>
						<Kbd>&uparrow;</Kbd><Kbd>&rightarrow;</Kbd>
						<Kbd>ESC</Kbd>
					</div>
					<div>
						<span class="text-secondary text-xs mr-2">Add to selection:</span>
						<Kbd>&DoubleUpArrow;</Kbd>+<Kbd>click</Kbd>
					</div>
				</div>
			</PanelSection>
		{/if}
	</div>
{/if}
