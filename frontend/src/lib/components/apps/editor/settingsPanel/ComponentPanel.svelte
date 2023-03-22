<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import {
		faChevronDown,
		faChevronRight,
		faChevronUp,
		faCopy,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext, RichConfiguration } from '../../types'
	import PanelSection from './common/PanelSection.svelte'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import TableActions from './TableActions.svelte'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'
	import ConnectedInputEditor from './inputEditor/ConnectedInputEditor.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { capitalize, classNames, getModifierKey, isMac } from '$lib/utils'
	import { buildExtraLib, fieldTypeToTsType } from '../../utils'
	import Recompute from './Recompute.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ComponentInputTypeEditor from './ComponentInputTypeEditor.svelte'
	import AlignmentEditor from './AlignmentEditor.svelte'
	import RunnableInputEditor from './inputEditor/RunnableInputEditor.svelte'
	import TemplateEditor from '$lib/components/TemplateEditor.svelte'
	import { ccomponents, components, type AppComponent } from '../component'
	import CssProperty from '../componentsPanel/CssProperty.svelte'
	import GridTab from './GridTab.svelte'
	import { deleteGridItem } from '../appUtils'
	import GridPane from './GridPane.svelte'
	import { slide } from 'svelte/transition'
	import { push } from '$lib/history'
	import Kbd from '$lib/components/common/kbd/Kbd.svelte'
	import { secondaryMenu } from './secondaryMenu'
	import StylePanel from './StylePanel.svelte'
	import { Delete } from 'lucide-svelte'

	export let component: AppComponent
	export let rowColumns = false
	export let onDelete: (() => void) | undefined = undefined
	export let parent: string | undefined
	export let noGrid = false
	export let duplicateMoveAllowed = true

	let editor: TemplateEditor | undefined = undefined

	const { app, runnableComponents, selectedComponent, worldStore, focusedGrid, stateId, state } =
		getContext<AppViewerContext>('AppViewerContext')

	const { history, ontextfocus } = getContext<AppEditorContext>('AppEditorContext')

	$: editor && ($ontextfocus = () => editor?.focus())

	function removeGridElement() {
		push(history, $app)
		$selectedComponent = undefined
		$focusedGrid = undefined
		if (component && !noGrid) {
			let ids = deleteGridItem($app, component, parent, false)
			for (const key of ids) {
				delete $runnableComponents[key]
			}
		}

		delete $runnableComponents[component.id]
		$app = $app
		$runnableComponents = $runnableComponents

		onDelete?.()
	}

	let viewCssOptions = false

	$: extraLib =
		component?.componentInput?.type === 'template' && $worldStore
			? buildExtraLib($worldStore?.outputsById ?? {}, component?.id, false, $state, false)
			: undefined

	function keydown(event: KeyboardEvent) {
		const { key, metaKey } = event
		if (key === 'Delte' || (key === 'Backspace' && metaKey)) {
			removeGridElement()
			event.stopPropagation()
		}
	}

	const initialConfiguration = ccomponents[component.type].initialData.configuration
	const componentInput: RichConfiguration | undefined =
		ccomponents[component.type].initialData.componentInput
</script>

<svelte:window on:keydown={keydown} />

{#if component}
	<div class="flex min-h-full flex-col min-w-[150px] w-full divide-y">
		{#if component.componentInput}
			<PanelSection title="Data Source">
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
						<StaticInputEditor
							fieldType={componentInput?.fieldType}
							subFieldType={componentInput?.subFieldType}
							format={componentInput?.format}
							bind:componentInput={component.componentInput}
							noVariablePicker
						/>
					{:else if component.componentInput.type === 'template' && component.componentInput !== undefined}
						<div class="py-1 min-h-[28px]  rounded border border-1 border-gray-500">
							<TemplateEditor
								bind:this={editor}
								fontSize={12}
								bind:code={component.componentInput.eval}
								{extraLib}
							/>
						</div>
					{:else if component.componentInput.type === 'connected' && component.componentInput !== undefined}
						<ConnectedInputEditor bind:componentInput={component.componentInput} />
					{:else if component.componentInput?.type === 'runnable' && component.componentInput !== undefined}
						<RunnableInputEditor
							appComponent={component}
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
										userInputEnabled={component.type === 'formcomponent' ||
											component.type === 'formbuttoncomponent'}
										{rowColumns}
									/>
								</PanelSection>
							</div>
						{/if}
					{/if}
				{/key}
			</PanelSection>
		{/if}
		{#if Object.values(initialConfiguration).length > 0}
			<PanelSection title={`Configuration (${Object.values(initialConfiguration).length})`}>
				<InputsSpecsEditor
					{rowColumns}
					id={component.id}
					inputSpecsConfiguration={initialConfiguration}
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

		{#if Object.keys(ccomponents[component.type].customCss ?? {}).length > 0}
			<PanelSection title="Custom CSS">
				<div slot="action">
					<Button
						color="light"
						size="xs"
						variant="border"
						endIcon={{ icon: faChevronRight }}
						on:click={() => secondaryMenu.open(StylePanel, { component })}
					>
						Open
					</Button>
				</div>
				{#if viewCssOptions}
					<div transition:slide|local class="w-full">
						{#each Object.keys(ccomponents[component.type].customCss ?? {}) as name}
							{#if component?.customCss != undefined}
								<div class="w-full pb-2">
									<CssProperty
										forceStyle={ccomponents[component.type].customCss[name].style != undefined}
										forceClass={ccomponents[component.type].customCss[name].class != undefined}
										{name}
										bind:value={component.customCss[name]}
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
						<span class="text-gray-600 text-xs mr-2"> Copy:</span>
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>C</Kbd>,
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>V</Kbd>
					</div>
					<div>
						<span class="text-gray-600 text-xs mr-2">Move: </span>
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>X</Kbd>,
						<Kbd>{getModifierKey()}</Kbd>+<Kbd>V</Kbd>
					</div>
					<div>
						<span class="text-gray-600 text-xs mr-2">Navigate:</span>
						<Kbd>&leftarrow;</Kbd>
						<Kbd>&uparrow;</Kbd><Kbd>&rightarrow;</Kbd>
						<Kbd>ESC</Kbd>
					</div>
				</div>
			</PanelSection>
		{/if}
	</div>
{/if}
