<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import { Badge, Button } from '../common'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import type { PickableProperties } from '../flows/previousResults'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import { filterNestedObject } from '../flows/previousResults'
	import type { PropPickerContext } from '../prop_picker'
	import Scrollable from '../Scrollable.svelte'

	export let pickableProperties: PickableProperties
	export let displayContext = true
	export let error: boolean = false
	export let allowCopy = false
	export let alwaysOn = false
	export let previousId: string | undefined = undefined

	let variables: Record<string, string> = {}
	let resources: Record<string, any> = {}
	let displayVariable = false
	let displayResources = false

	let allResultsCollapsed = true
	let collapsableInitialState:
		| {
				allResultsCollapsed: boolean
				displayVariable: boolean
				displayResources: boolean
		  }
		| undefined

	let filterActive = false

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig, inputMatches } =
		getContext<PropPickerWrapperContext>('PropPickerWrapper')

	const { pickablePropertiesFiltered } = getContext<PropPickerContext>('PropPickerContext')
	$: $pickablePropertiesFiltered = pickableProperties

	let flowInputsFiltered: any = pickableProperties.flow_input
	let resultByIdFiltered: any = pickableProperties.priorIds

	let timeout: NodeJS.Timeout | undefined
	function onSearch(search: string) {
		filterActive = false
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
		}, 50)
	}

	$: suggestedPropsFiltered =
		$propPickerConfig && $propPickerConfig.propName
			? keepByKey(pickableProperties.priorIds, $propPickerConfig.propName ?? '')
			: undefined
	$: search != undefined && onSearch(search)

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

	let filteringFlowInputsOrResult = ''
	async function filterPickableProperties() {
		if (!$propPickerConfig || !filterActive) {
			if (search === EMPTY_STRING) {
				flowInputsFiltered = pickableProperties.flow_input
				resultByIdFiltered = pickableProperties.priorIds
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
			collapsableInitialState = { allResultsCollapsed, displayVariable, displayResources }
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
		if ($inputMatches[0].word === 'results') {
			allResultsCollapsed = false
			return
		}
	}

	function resetCollapsable() {
		if (!collapsableInitialState) {
			return
		}
		;({ allResultsCollapsed, displayVariable, displayResources } = collapsableInitialState)
		collapsableInitialState = undefined
	}

	async function updateFilterActive() {
		const prev = filterActive

		filterActive = Boolean(
			$inputMatches &&
				$inputMatches?.length > 0 &&
				$propPickerConfig?.insertionMode === 'insert' &&
				search === EMPTY_STRING
		)

		if (prev && !filterActive) {
			flowInputsFiltered = pickableProperties.flow_input
			resultByIdFiltered = pickableProperties.priorIds
		}
	}

	async function updateState() {
		await updateFilterActive()
		await filterPickableProperties()
		await updateCollapsable()
	}

	$: search, $inputMatches, $propPickerConfig, pickableProperties, updateState()
</script>

<div class="flex flex-col h-full rounded">
	<div class="px-1 pb-1">
		<ClearableInput bind:value={search} placeholder="Search prop..." />
	</div>
	<!-- <div
	class="px-2 pt-2 grow relative"
	class:bg-surface-secondary={!$propPickerConfig && !alwaysOn} -->
	<Scrollable
		scrollableClass="grow relative min-h-0 px-1 xl:px-2 py-1 shrink {!$propPickerConfig && !alwaysOn
			? 'bg-surface-secondary'
			: ''}"
	>
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
		{#if flowInputsFiltered && (Object.keys(flowInputsFiltered ?? {}).length > 0 || !filterActive)}
			<div class="flex justify-between items-center space-x-1">
				<span class="font-normal text-sm text-secondary"
					><span class="font-mono">flow_input</span></span
				>
				<div class="flex space-x-2 items-center"></div>
			</div>
			<div class="overflow-y-auto pb-2">
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
			<span class="font-normal text-sm text-secondary">Error</span>
			<div class="overflow-y-auto pb-2">
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
					<span class="font-normal text-sm text-secondary">Suggested Results</span>
					<div class="overflow-y-auto pb-2">
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
				<span class="font-normal text-sm text-tertiary font-mono">results</span>
				<div class="overflow-y-auto pb-2">
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
				<span class="font-normal text-sm text-secondary">Resume payloads</span>
				<div class="overflow-y-auto pb-2">
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
				{#if !filterActive && suggestedPropsFiltered && Object.keys(suggestedPropsFiltered).length > 0}
					<span class="font-normal text-sm text-secondary">Suggested Results</span>
					<div class="overflow-y-auto pb-2">
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
					<div class="overflow-y-auto pb-2 pt-2">
						<span class="font-normal text-sm text-tertiary font-mono">results</span>

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
			{#if !filterActive || $inputMatches?.some((match) => match.word === 'variable')}
				<div class="overflow-y-auto pb-2 pt-4">
					<span class="font-normal text-xs text-secondary">Variables:</span>

					{#if displayVariable}
						<Button
							color="light"
							size="xs2"
							variant="border"
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
							color="light"
							size="xs2"
							variant="border"
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
			{#if !filterActive || $inputMatches?.some((match) => match.word === 'resource')}
				<div class="overflow-y-auto pb-2">
					<span class="font-normal text-xs text-secondary">Resources:</span>

					{#if displayResources}
						<Button
							color="light"
							size="xs2"
							variant="border"
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
							color="light"
							size="xs2"
							variant="border"
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
		{/if}
		<!-- </div> -->
	</Scrollable>
</div>
