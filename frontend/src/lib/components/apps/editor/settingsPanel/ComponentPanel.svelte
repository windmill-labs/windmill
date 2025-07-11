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
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ComponentInputTypeEditor from './ComponentInputTypeEditor.svelte'
	import AlignmentEditor from './AlignmentEditor.svelte'
	import RunnableInputEditor from './inputEditor/RunnableInputEditor.svelte'
	import TemplateEditor from '$lib/components/TemplateEditor.svelte'
	import { ccomponents, components } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import GridTab from './GridTab.svelte'
	import { deleteGridItem, isTableAction } from '../appUtils'
	import GridPane from './GridPane.svelte'
	import { slide } from 'svelte/transition'
	import { push } from '$lib/history.svelte'
	import StylePanel from './StylePanel.svelte'
	import { ChevronLeft, ArrowBigUp, ArrowLeft } from 'lucide-svelte'
	import GridCondition from './GridCondition.svelte'
	import { isTriggerable } from './script/utils'
	import { inferDeps } from '../appUtilsInfer'
	import EvalV2InputEditor from './inputEditor/EvalV2InputEditor.svelte'
	import type { ResultAppInput } from '../../inputType'
	import GridGroup from './GridGroup.svelte'
	import { secondaryMenuLeft } from './secondaryMenu'
	import DocLink from './DocLink.svelte'

	import ComponentControl from './ComponentControl.svelte'
	import GridAgGridLicenseKey from './GridAgGridLicenseKey.svelte'
	import ComponentPanelDataSource from './ComponentPanelDataSource.svelte'
	import MenuItems from './MenuItems.svelte'
	import DecisionTreeGraphEditor from './DecisionTreeGraphEditor.svelte'
	import GridAgChartsLicenseKe from './GridAgChartsLicenseKe.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ContextVariables from './ContextVariables.svelte'
	import EventHandlers from './EventHandlers.svelte'
	import GridNavbar from './GridNavbar.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { twMerge } from 'tailwind-merge'
	import Popover from '$lib/components/Popover.svelte'

	interface Props {
		parent: string | undefined
		item: GridItem
		onDelete?: (() => void) | undefined
		noGrid?: boolean
		duplicateMoveAllowed?: boolean
	}

	let {
		parent,
		item = $bindable(),
		onDelete = undefined,
		noGrid = false,
		duplicateMoveAllowed = true
	}: Props = $props()

	const {
		app,
		runnableComponents,
		selectedComponent,
		worldStore,
		focusedGrid,
		stateId,
		state: stateStore,
		errorByComponent,
		componentControl
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history, movingcomponents } = getContext<AppEditorContext>('AppEditorContext')

	function removeGridElement() {
		push(history, $app)

		const id = item?.id
		const onDeleteComponentControl = id ? $componentControl[id]?.onDelete : undefined
		if (onDeleteComponentControl) {
			onDeleteComponentControl()
		}
		if (onDelete) {
			onDelete()
		}

		let cId = item?.id
		if (cId) {
			delete $worldStore.outputsById[cId]
			delete $errorByComponent[cId]

			if ($movingcomponents?.includes(cId)) {
				$movingcomponents = $movingcomponents.filter((id) => id !== cId)
			}
		}
		const nitem = item
		const nparent = parent
		$selectedComponent = undefined
		$focusedGrid = undefined
		if (nitem && !noGrid) {
			let ids = deleteGridItem($app, nitem.data, nparent)
			for (const key of ids) {
				delete $runnableComponents[key]
			}
		}

		if (nitem?.data?.id) {
			delete $runnableComponents[nitem?.data?.id]
		}
		$app = $app
		$runnableComponents = $runnableComponents

		onDelete?.()
	}

	let viewCssOptions = false

	let extraLib = $derived(
		(item?.data?.componentInput?.type === 'template' ||
			item?.data?.componentInput?.type === 'templatev2') &&
			$worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, item?.data?.id, $stateStore, false)
			: undefined
	)

	// 	`
	// /** The current's app state */
	// const state: Record<string, any> = ${JSON.stringify(state)};`

	function keydown(event: KeyboardEvent) {
		const { key, metaKey } = event
		if (key === 'Delete' || (key === 'Backspace' && metaKey)) {
			removeGridElement()
			event.stopPropagation()
		}
	}

	const initialConfiguration = item?.data?.type
		? ccomponents[item.data.type]?.initialData?.configuration
		: {}

	const componentInput: RichConfiguration | undefined = item?.data?.type
		? ccomponents[item?.data?.type]?.initialData?.componentInput
		: undefined

	const hasInteraction = item.data.type ? isTriggerable(item.data.type) : false

	let evalV2editor: EvalV2InputEditor | undefined = $state(undefined)

	function transformToFrontend() {
		if (item.data.componentInput) {
			const id = item?.data?.id
			let appInput: ResultAppInput = {
				...item.data.componentInput,
				type: 'runnable',
				runnable: {
					type: 'runnableByName',
					name: `Eval of ${id}`,
					inlineScript: {
						content: `return ${item.data.componentInput?.['expr']}`,
						language: 'frontend',
						refreshOn: item.data.componentInput?.['connections']?.map((c) => {
							return {
								id: c.componentId,
								key: c.id
							}
						})
					}
				},
				fields: {}
			}
			item.data.componentInput = appInput
		}
	}

	let templateChangeTimeout: NodeJS.Timeout | undefined = undefined
	function onTemplateChange(e: CustomEvent<{ code: string }>) {
		const currentItem = item
		if (currentItem.data.componentInput?.type === 'templatev2') {
			if (templateChangeTimeout) {
				clearTimeout(templateChangeTimeout)
			}
			templateChangeTimeout = setTimeout(() => {
				if (currentItem.data.componentInput?.type === 'templatev2') {
					inferDeps(
						'`' + e.detail.code + '`',
						$worldStore.outputsById,
						currentItem.data.componentInput,
						app
					)
					console.log('inferred deps for', currentItem.data.id)
				}
			}, 200)
		}
	}
</script>

<svelte:window onkeydown={keydown} />

{#if item?.id && isTableAction(item?.id, $app)}
	<div
		class="flex items-center px-3 py-2 bg-surface border-b text-xs font-semibold gap-2 justify-between"
	>
		<div class="flex flex-row items-center gap-2">
			<Popover>
				{#snippet text()}
					<div class="flex flex-row gap-1"> Back to table component </div>
				{/snippet}
				<Button
					iconOnly
					startIcon={{
						icon: ArrowLeft
					}}
					size="xs"
					btnClasses={twMerge(
						'p-1 text-gray-300 hover:!text-gray-600 dark:text-gray-500 dark:hover:!text-gray-200 bg-transparent'
					)}
					on:click={() => {
						const tableId = item?.id?.split?.('_')?.[0]

						if (tableId) {
							$selectedComponent = [tableId]
						}
					}}
					color="light"
				/>
			</Popover>

			<div class="flex flex-row gap-2 items-center">
				Table action of table
				<Badge color="indigo">{item?.id.split('_')[0]}</Badge>
			</div>
		</div>

		<DocLink
			docLink="https://www.windmill.dev/docs/apps/app_configuration_settings/aggrid_table#table-actions"
		/>
	</div>
{/if}

{#if item?.data}
	{@const component = item.data}
	<div class="flex justify-between items-center px-3 py-1 bg-surface-secondary">
		<div class="text-xs text-primary font-semibold"
			>{components[item.data.type]?.name ?? 'Unknown'}</div
		>
		<DocLink docLink={components[item.data.type]?.documentationLink} />
	</div>

	<div class="flex min-h-[calc(100%-32px)] flex-col min-w-[150px] w-full divide-y">
		<ComponentPanelDataSource bind:component={item.data}>
			{#if component.componentInput}
				<PanelSection
					title={item.data.type == 'steppercomponent'
						? 'Validations'
						: item.data.type == 's3fileinputcomponent'
							? 'Path template'
							: hasInteraction
								? 'Event handler'
								: 'Data source'}
					id={'component-input'}
				>
					{#snippet action()}
						<div class="flex flex-row gap-1 justify-center items-center">
							<DocLink
								docLink={'https://www.windmill.dev/docs/apps/app-runnable-panel#creating-a-runnable'}
							/>
							<div
								class={classNames(
									'text-white px-2 text-2xs py-0.5 font-bold rounded-sm',
									'bg-indigo-500'
								)}
							>
								{`${component.id}`}
							</div>
						</div>
					{/snippet}

					{#if item.data.componentInput}
						<ComponentInputTypeEditor
							{evalV2editor}
							bind:componentInput={item.data.componentInput}
						/>

						<div class="flex flex-col w-full gap-2 mt-2">
							{#if item.data.componentInput.type === 'static'}
								<StaticInputEditor
									id={component.id}
									fieldType={componentInput?.fieldType}
									subFieldType={componentInput?.subFieldType}
									format={componentInput?.format}
									bind:componentInput={item.data.componentInput}
								/>
							{:else if item.data?.componentInput?.type === 'template' || item.data?.componentInput?.type === 'templatev2'}
								<div class="py-1 min-h-[28px] rounded border border-1 border-gray-500">
									<TemplateEditor
										fontSize={12}
										bind:code={item.data.componentInput.eval}
										{extraLib}
										on:change={onTemplateChange}
									/>
								</div>
								{#if item.data?.componentInput?.type === 'templatev2'}
									{#if item.data?.componentInput.connections?.length > 0}
										<div class="flex flex-wrap gap-2 items-center">
											<div class="text-2xs text-tertiary">Re-evaluated on changes to:</div>
											<div class="flex flex-wrap gap-1">
												{#each item.data?.componentInput.connections ?? [] as connection (connection.componentId + '-' + connection.id)}
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
							{:else if item.data.componentInput.type === 'connected' && component.componentInput !== undefined}
								<ConnectedInputEditor bind:componentInput={item.data.componentInput} />
							{:else if item.data.componentInput.type === 'evalv2' && component.componentInput !== undefined}
								<EvalV2InputEditor
									field="nonrunnable"
									bind:this={evalV2editor}
									id={component.id}
									bind:componentInput={item.data.componentInput}
								/>
								<a class="text-2xs" onclick={transformToFrontend} href={undefined}>
									transform to a frontend script
								</a>
							{:else if item.data.componentInput?.type === 'runnable' && component.componentInput !== undefined}
								<RunnableInputEditor
									appComponent={component}
									bind:appInput={item.data.componentInput}
									defaultUserInput={component.type == 'formcomponent' ||
										component.type == 'formbuttoncomponent'}
								/>
							{/if}
						</div>
					{/if}

					<ContextVariables type={component.type} id={component.id} />

					{#key $stateId}
						{#if item.data.componentInput?.type === 'runnable'}
							{#if Object.keys(item.data.componentInput.fields ?? {}).length > 0}
								<div class="w-full">
									<div class="flex flex-row items-center gap-2 text-sm font-semibold">
										Runnable Inputs

										<Tooltip>
											The runnable inputs are inferred from the inputs of the flow or script
											parameters this component is attached to.
										</Tooltip>
									</div>
									<InputsSpecsEditor
										id={component.id}
										shouldCapitalize={false}
										displayType
										bind:inputSpecs={item.data.componentInput.fields}
										userInputEnabled={component.type === 'formcomponent' ||
											component.type === 'formbuttoncomponent'}
										recomputeOnInputChanged={item.data.componentInput.recomputeOnInputChanged}
										showOnDemandOnlyToggle
										acceptSelf={component.type === 'aggridinfinitecomponent' ||
											component.type === 'aggridinfinitecomponentee' ||
											component.type === 'steppercomponent'}
										overridenByComponent={component.type === 'aggridinfinitecomponent' ||
										component.type === 'aggridinfinitecomponentee'
											? ['offset', 'limit', 'orderBy', 'isDesc', 'search']
											: []}
										securedContext
									/>
								</div>
							{/if}
						{/if}
					{/key}
				</PanelSection>
			{/if}
		</ComponentPanelDataSource>
		<ComponentControl type={component.type} />

		{#if item.data.type === 'navbarcomponent'}
			<GridNavbar bind:navbarItems={item.data.navbarItems} id={component.id} />
		{/if}
		{#if item.data.type === 'tabscomponent'}
			<GridTab
				bind:tabs={item.data.tabs}
				bind:disabledTabs={item.data.disabledTabs}
				bind:component={item.data}
				canDisableTabs
			/>
		{:else if item.data.type === 'aggridcomponentee'}
			<GridAgGridLicenseKey bind:license={item.data.license} />
			<TableActions
				id={component.id}
				bind:components={item.data.actions}
				bind:actionsOrder={item.data.actionsOrder}
			/>
		{:else if item.data.type === 'agchartscomponentee'}
			<GridAgChartsLicenseKe bind:license={item.data.license} />
		{:else if item.data.type === 'steppercomponent'}
			<GridTab bind:tabs={item.data.tabs} bind:component={item.data} word="Step" />
		{:else if item.data.type === 'containercomponent'}
			<GridGroup bind:groupFields={item.data.groupFields} {item} />
		{:else if item.data.type === 'conditionalwrapper'}
			<GridCondition bind:conditions={item.data.conditions} bind:component={item.data} />
		{:else if item.data.type === 'decisiontreecomponent'}
			<DecisionTreeGraphEditor bind:nodes={item.data.nodes} bind:component={item.data} />
		{:else if item.data.type === 'verticalsplitpanescomponent' || item.data.type === 'horizontalsplitpanescomponent'}
			<GridPane bind:panes={item.data.panes} bind:component={item.data} />
		{:else if item.data.type === 'aggridcomponent'}
			<TableActions
				id={component.id}
				bind:components={item.data.actions}
				bind:actionsOrder={item.data.actionsOrder}
			/>
		{:else if item.data.type === 'aggridinfinitecomponent'}
			<TableActions
				id={component.id}
				bind:components={item.data.actions}
				bind:actionsOrder={item.data.actionsOrder}
			/>
		{:else if item.data.type === 'aggridinfinitecomponentee'}
			<GridAgGridLicenseKey bind:license={item.data.license} />
			<TableActions
				id={component.id}
				bind:components={item.data.actions}
				bind:actionsOrder={item.data.actionsOrder}
			/>
		{:else if item.data.type === 'dbexplorercomponent'}
			<TableActions
				id={component.id}
				bind:components={item.data.actions}
				bind:actionsOrder={item.data.actionsOrder}
			/>
		{:else if item.data.type === 'tablecomponent' && Array.isArray(item.data.actionButtons)}
			<TableActions id={component.id} bind:components={item.data.actionButtons} />
		{:else if item.data.type === 'menucomponent' && Array.isArray(item.data.menuItems)}
			<MenuItems id={component.id} bind:components={item.data.menuItems} />
		{/if}

		{#if Object.values(initialConfiguration).length > 0}
			<PanelSection title="Configuration">
				<InputsSpecsEditor
					id={component.id}
					inputSpecsConfiguration={initialConfiguration}
					bind:inputSpecs={item.data.configuration}
					userInputEnabled={false}
					acceptSelf
				/>
			</PanelSection>
		{:else if item.data.type != 'containercomponent'}
			<div class="h-full w-full text-sm text-tertiary text-center py-8 px-2">
				{ccomponents[component.type].name} has no configuration
			</div>
		{/if}

		<EventHandlers bind:item ownId={component.id} />

		<div class="grow shrink"></div>

		{#if Object.keys(ccomponents[component.type]?.customCss ?? {}).length > 0}
			<PanelSection title="Styling">
				{#snippet action()}
					<div class="flex justify-end flex-wrap gap-1">
						<Button
							color="light"
							size="xs"
							variant="border"
							startIcon={{ icon: ChevronLeft }}
							on:click={() => secondaryMenuLeft.toggle(StylePanel, { type: 'style' })}
						>
							Show
						</Button>
					</div>
				{/snippet}
				<div class="flex gap-2 items-center flex-wrap">
					<div class="!text-2xs">Full height</div>
					{#if item?.[12]?.fullHeight !== undefined}
						<Toggle
							bind:checked={item[12].fullHeight}
							size="xs"
							options={{
								right: 'Desktop'
							}}
						/>
					{/if}
					{#if item?.[3]?.fullHeight !== undefined}
						<Toggle
							bind:checked={item[3].fullHeight}
							size="xs"
							options={{
								right: 'Mobile'
							}}
						/>
					{/if}
				</div>
				<AlignmentEditor bind:component={item.data} />
				{#if viewCssOptions}
					<div transition:slide|local class="w-full">
						{#each Object.keys(ccomponents[component.type]?.customCss ?? {}) as name}
							{#if item.data?.customCss != undefined}
								<div class="w-full mb-2">
									<CssProperty
										forceStyle={ccomponents[component.type].customCss[name].style != undefined}
										forceClass={ccomponents[component.type].customCss[name].class != undefined}
										tooltip={ccomponents[component.type].customCss[name].tooltip}
										{name}
										bind:value={item.data.customCss[name]}
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
				{#snippet action()}
					<div>
						<Button
							size="xs"
							color="red"
							variant="border"
							on:click={removeGridElement}
							shortCut={{
								key: isMac() ? getModifierKey() + 'Del' : 'Del',
								withoutModifier: true
							}}
						>
							Delete
						</Button>
					</div>
				{/snippet}

				<div class="overflow-auto grid grid-cols-2 gap-1 text-tertiary">
					<div>
						<span class="text-secondary text-xs">Copy:</span>
					</div>
					<div class="flex items-center gap-1">
						<div class="text-xs border py-1 px-1.5 rounded-md">{getModifierKey() + 'C'}</div>
						<span class="text-xs">&rightarrow;</span>
						<span class="text-xs border py-1 px-1.5 rounded-md">{getModifierKey() + 'V'}</span>
					</div>

					<div>
						<span class="text-secondary text-xs">Move: </span>
					</div>
					<div class="flex items-center gap-1">
						<div class="text-xs border py-1 px-1.5 rounded-md">{getModifierKey() + 'X'}</div>
						<span class="text-xs">&rightarrow;</span>
						<span class="text-xs border py-1 px-1.5 rounded-md">{getModifierKey() + 'V'}</span>
					</div>

					<div>
						<span class="text-secondary text-xs">Navigate:</span>
					</div>
					<div class="flex items-center gap-1">
						<span class="text-xs border py-1 px-1.5 rounded-md">&leftarrow;</span>
						<span class="text-xs border py-1 px-1.5 rounded-md">&uparrow;</span>
						<span class="text-xs border py-1 px-1.5 rounded-md">&rightarrow;</span>
						<span class="text-xs border py-1 px-1.5 rounded-md">ESC</span>
					</div>

					<div>
						<span class="text-secondary text-xs whitespace-nowrap">Add to selection:</span>
					</div>
					<div class="flex items-center gap-1">
						<span class="text-xs border py-1 px-1.5 rounded-md">
							<ArrowBigUp size="14" />
						</span>+<span class="text-xs border py-1 px-1.5 rounded-md">Click</span>
					</div>
				</div>
			</PanelSection>
		{/if}
	</div>
{/if}
