<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext, onDestroy, untrack } from 'svelte'
	import { Badge, Button } from '../common'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import type { PickableProperties } from '../flows/previousResults'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import { filterNestedObject } from '../flows/previousResults'
	import type { PropPickerContext } from '../prop_picker'
	import Scrollable from '../Scrollable.svelte'

	interface Props {
		pickableProperties: PickableProperties
		displayContext?: boolean
		error?: boolean
		allowCopy?: boolean
		previousId?: string | undefined
		flow_env?: Record<string, any> | undefined
		result?: any | undefined
		extraResults?: any
	}

	let {
		pickableProperties,
		displayContext = true,
		error = false,
		allowCopy = false,
		previousId = undefined,
		flow_env = undefined,
		result = undefined,
		extraResults = undefined
	}: Props = $props()

	let variables: Record<string, string> = $state({})
	let resources: Record<string, any> = $state({})
	let displayVariable = $state(false)
	let displayResources = $state(false)
	let displayFlowEnv = $state(false)

	let allResultsCollapsed = $state(true)
	let collapsableInitialState:
		| {
				allResultsCollapsed: boolean
				displayVariable: boolean
				displayResources: boolean
				displayFlowEnv: boolean
		  }
		| undefined

	const EMPTY_STRING = ''
	let search = $state('')

	const {
		propPickerConfig,
		inputMatches,
		exprBeingEdited: propsBeingEdited
	} = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	const { pickablePropertiesFiltered } = getContext<PropPickerContext>('PropPickerContext')
	$effect(() => {
		$pickablePropertiesFiltered = pickableProperties
	})

	let flowInputsFiltered: any = $state(pickableProperties.flow_input)
	let resultByIdFiltered: any = $state(pickableProperties.priorIds)
	let flowEnvFiltered: any = $state(pickableProperties.flow_env)

	let timeout: number | undefined
	function onSearch(search: string) {
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(() => {
			flowInputsFiltered =
				search === EMPTY_STRING
					? pickableProperties.flow_input
					: keepByKey(pickableProperties.flow_input, search)

			resultByIdFiltered =
				search === EMPTY_STRING
					? pickableProperties.priorIds
					: keepByKey(pickableProperties.priorIds, search)

			flowEnvFiltered =
				search === EMPTY_STRING
					? pickableProperties.flow_env
					: keepByKey(pickableProperties.flow_env, search)
		}, 50)
	}

	let suggestedPropsFiltered = $derived(
		$propPickerConfig && $propPickerConfig.propName
			? keepByKey(pickableProperties.priorIds, $propPickerConfig.propName ?? '')
			: undefined
	)
	$effect(() => {
		search != undefined && untrack(() => onSearch(search))
	})

	async function loadVariables() {
		variables = Object.fromEntries(
			(
				await VariableService.listVariable({
					workspace: $workspaceStore ?? ''
				})
			).map((variable) => [variable.path, variable.is_secret ? '***' : (variable.value ?? '')])
		)
	}

	async function loadResources() {
		resources = Object.fromEntries(
			(
				await ResourceService.listResource({
					workspace: $workspaceStore ?? ''
				})
			).map((resource) => [resource.path, resource.description ?? ''])
		)
	}

	let filteringFlowInputsOrResult = $state('')
	async function filterPickableProperties() {
		if ($propsBeingEdited?.length == 0 || $propPickerConfig || !$inputMatches?.length) {
			if (search === EMPTY_STRING) {
				flowInputsFiltered = pickableProperties.flow_input
				resultByIdFiltered = pickableProperties.priorIds
				flowEnvFiltered = pickableProperties.flow_env
			}
			filteringFlowInputsOrResult = ''
			return
		}
		if (!$inputMatches?.some((match) => match.word === 'flow_input')) {
			flowInputsFiltered = {}
		}
		if (!$inputMatches?.some((match) => match.word === 'results')) {
			resultByIdFiltered = {}
		}
		if (!$inputMatches?.some((match) => match.word === 'flow_env')) {
			flowEnvFiltered = {}
		}
		if ($inputMatches?.length == 1) {
			filteringFlowInputsOrResult = $inputMatches[0].value
			if ($inputMatches[0].word === 'flow_input') {
				flowInputsFiltered = pickableProperties.flow_input
				let [, ...nestedKeys] = $inputMatches[0].value.split('.')
				let filtered = filterNestedObject(flowInputsFiltered, nestedKeys)
				if (Object.keys(filtered).length > 0) {
					flowInputsFiltered = filtered
				}
			} else if ($inputMatches[0].word === 'results') {
				resultByIdFiltered = pickableProperties.priorIds
				let [, ...nestedKeys] = $inputMatches[0].value.split('.')
				let filtered = filterNestedObject(resultByIdFiltered, nestedKeys)
				if (Object.keys(filtered).length > 0) {
					resultByIdFiltered = filtered
				}
			} else if ($inputMatches[0].word === 'flow_env') {
				flowEnvFiltered = pickableProperties.flow_env
				let [, ...nestedKeys] = $inputMatches[0].value.split('.')
				let filtered = filterNestedObject(flowEnvFiltered, nestedKeys)
				if (Object.keys(filtered).length > 0) {
					flowEnvFiltered = filtered
				}
			}
		} else {
			filteringFlowInputsOrResult = ''
		}

		// if ($pickablePropertiesFiltered) {
		// 	resultByIdFiltered && ($pickablePropertiesFiltered.priorIds = resultByIdFiltered)
		// 	flowInputsFiltered && ($pickablePropertiesFiltered.flow_input = flowInputsFiltered)
		// }
	}

	async function updateCollapsable() {
		if (!$inputMatches || $inputMatches.length !== 1) {
			resetCollapsable()
			return
		}

		if (!collapsableInitialState) {
			collapsableInitialState = {
				allResultsCollapsed,
				displayVariable,
				displayResources,
				displayFlowEnv
			}
		}

		if ($inputMatches[0].word === 'variable') {
			await loadVariables()
			displayVariable = true
			return
		}
		if ($inputMatches[0].word === 'resource') {
			await loadResources()
			displayResources = true
			return
		}
		if ($inputMatches[0].word === 'flow_env') {
			displayFlowEnv = true
			return
		}
		if ($inputMatches[0].word === 'results') {
			allResultsCollapsed = false
			return
		}
	}

	function resetCollapsable() {
		if (!collapsableInitialState) {
			return
		}
		;({ allResultsCollapsed, displayVariable, displayResources, displayFlowEnv } =
			collapsableInitialState)
		collapsableInitialState = undefined
	}

	async function updateState() {
		await filterPickableProperties()
		await updateCollapsable()
	}

	$effect(() => {
		;[search, $inputMatches, $propPickerConfig, pickableProperties, $propsBeingEdited]
		untrack(() => updateState())
	})

	onDestroy(() => {
		clearTimeout(timeout)
	})

	const categoryContentClasses = 'overflow-y-auto pb-4'
	const categoryTitleClasses = 'font-semibold text-xs text-emphasis'
</script>

<div class="flex flex-col h-full">
	<div class="px-1 pb-1">
		<ClearableInput bind:value={search} placeholder="Search prop..." />
	</div>
	<!-- <div
	class="px-2 pt-2 grow relative"
	class:bg-surface-secondary={!$propPickerConfig && !alwaysOn} -->
	<Scrollable scrollableClass="grow relative min-h-0 px-1 xl:px-2 py-1 shrink">
		<!-- <div
			class="px-2 pt-2 grow relative"
			class:bg-surface-secondary={!$propPickerConfig && !alwaysOn}
		> -->
		<!-- <pre class="text-2xs w-full"
			>{JSON.stringify({ pickableProperties, resultByIdFiltered }, null, 2)}</pre
		> -->
		{#if filteringFlowInputsOrResult}
			<div class="absolute bottom-0 right-0">
				<Badge small>filter: {filteringFlowInputsOrResult}</Badge>
			</div>
		{/if}
		{#if result != undefined}
			<span class={categoryTitleClasses}>Step Result</span>
			<div class={categoryContentClasses}>
				<ObjectViewer
					{allowCopy}
					json={{ result, ...(extraResults ? extraResults : {}) }}
					on:select
				/>
			</div>
		{/if}
		{#if flowInputsFiltered && Object.keys(flowInputsFiltered ?? {}).length > 0}
			<span class={categoryTitleClasses}>Flow Input</span>
			<div class={categoryContentClasses}>
				<ObjectViewer
					{allowCopy}
					pureViewer={!$propPickerConfig}
					json={flowInputsFiltered}
					prefix="flow_input"
					on:select
				/>
			</div>
		{/if}
		{#if error}
			<span class={categoryTitleClasses}>Error</span>
			<div class={categoryContentClasses}>
				<ObjectViewer
					{allowCopy}
					pureViewer={!$propPickerConfig}
					json={{
						error: {
							message: 'The error message',
							name: 'The error name',
							stack: 'The error stack',
							step_id: 'The step id'
						}
					}}
					on:select
				/>
			</div>
			{#if Object.keys(pickableProperties.priorIds).length > 0}
				{#if suggestedPropsFiltered && Object.keys(suggestedPropsFiltered).length > 0}
					<span class={categoryTitleClasses}>Suggested Results</span>
					<div class={categoryContentClasses}>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							collapsed={false}
							json={suggestedPropsFiltered}
							prefix="results"
							on:select
						/>
					</div>
				{/if}
				<span class={categoryTitleClasses}>Results</span>
				<div class={categoryContentClasses}>
					<ObjectViewer
						expandedEvenOnLevel0={previousId}
						{allowCopy}
						pureViewer={!$propPickerConfig}
						collapseLevel={allResultsCollapsed ? 1 : undefined}
						json={resultByIdFiltered}
						prefix="results"
						on:select
					/>
				</div>
			{/if}
		{:else}
			{#if pickableProperties.hasResume}
				<span class={categoryTitleClasses}>Resume payloads</span>
				<div class={categoryContentClasses}>
					<ObjectViewer
						{allowCopy}
						pureViewer={!$propPickerConfig}
						json={{
							resume: 'The resume payload',
							resumes: 'All resume payloads from all approvers',
							approvers: 'The list of approvers'
						}}
						on:select
					/>
				</div>
			{/if}
			{#if Object.keys(pickableProperties.priorIds).length > 0}
				{#if suggestedPropsFiltered && Object.keys(suggestedPropsFiltered).length > 0}
					<span class={categoryTitleClasses}>Suggested Results</span>
					<div class={categoryContentClasses}>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							collapsed={false}
							json={suggestedPropsFiltered}
							prefix="results"
							on:select
						/>
					</div>
				{/if}
				{#if Object.keys(resultByIdFiltered ?? {}).length > 0}
					<span class={categoryTitleClasses}>Results</span>
					<div class={categoryContentClasses}>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							collapseLevel={allResultsCollapsed ? 1 : undefined}
							json={resultByIdFiltered}
							expandedEvenOnLevel0={previousId}
							prefix="results"
							on:select
						/>
					</div>
				{/if}
			{/if}
		{/if}

		{#if displayContext}
			{#if $inputMatches?.some((match) => match.word === 'variable')}
				<span class={categoryTitleClasses}>Variables</span>
				<div class={categoryContentClasses}>
					{#if displayVariable}
						<Button
							size="xs2"
							variant="default"
							on:click={() => {
								displayVariable = false
							}}
							wrapperClasses="inline-flex whitespace-nowrap w-fit"
							btnClasses="font-mono h-4 text-2xs font-thin px-1 rounded-[0.275rem]">-</Button
						>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							rawKey={true}
							json={variables}
							prefix="variable"
							on:select
						/>
					{:else}
						<Button
							size="xs2"
							variant="default"
							on:click={async () => {
								await loadVariables()
								displayVariable = true
							}}
							wrapperClasses="inline-flex w-fit"
							btnClasses="font-normal text-2xs rounded-[0.275rem] h-4 px-1"
						>
							{'{...}'}
						</Button>
					{/if}
				</div>
			{/if}
			{#if $inputMatches?.some((match) => match.word === 'resource')}
				<span class={categoryTitleClasses}>Resources</span>
				<div class={categoryContentClasses}>
					{#if displayResources}
						<Button
							size="xs2"
							variant="default"
							on:click={() => {
								displayResources = false
							}}
							wrapperClasses="inline-flex whitespace-nowrap w-fit"
							btnClasses="font-mono h-4 text-2xs font-thin px-1 rounded-[0.275rem]">-</Button
						>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							rawKey={true}
							json={resources}
							prefix="resource"
							on:select
						/>
					{:else}
						<Button
							size="xs2"
							variant="default"
							on:click={async () => {
								await loadResources()
								displayResources = true
							}}
							wrapperClasses="inline-flex whitespace-nowrap w-fit"
							btnClasses="font-normal text-2xs rounded-[0.275rem] h-4 px-1"
						>
							{'{...}'}
						</Button>
					{/if}
				</div>
			{/if}
			{#if flow_env && Object.keys(flow_env).length > 0 && $inputMatches?.some((match) => match.word === 'flow_env')}
				<div class="overflow-y-auto pb-2">
					<span class="font-normal text-xs text-secondary">Flow Env Variables:</span>

					{#if displayFlowEnv}
						<Button
							color="light"
							size="xs2"
							variant="border"
							on:click={() => {
								displayFlowEnv = false
							}}
							wrapperClasses="inline-flex whitespace-nowrap w-fit"
							btnClasses="font-mono h-4 text-2xs font-thin px-1 rounded-[0.275rem]">-</Button
						>
						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							rawKey={false}
							json={flowEnvFiltered}
							prefix="flow_env"
							on:select
						/>
					{:else}
						<Button
							color="light"
							size="xs2"
							variant="border"
							on:click={() => {
								displayFlowEnv = true
							}}
							wrapperClasses="inline-flex whitespace-nowrap w-fit"
							btnClasses="font-normal text-2xs rounded-[0.275rem] h-4 px-1"
						>
							{'{...}'}
						</Button>
					{/if}
				</div>
			{/if}
		{/if}
		<!-- </div> -->
	</Scrollable>
</div>
