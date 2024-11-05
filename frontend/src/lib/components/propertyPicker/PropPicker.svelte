<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import { Badge, Button } from '../common'
	import type { PropPickerWrapperContext } from '../prop_picker'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import type { PickableProperties } from '../flows/previousResults'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import { filterNestedObject } from '../flows/previousResults'

	export let pickableProperties: PickableProperties
	export let displayContext = true
	export let notSelectable: boolean
	export let error: boolean = false
	export let allowCopy = false

	$: previousId = pickableProperties?.previousId
	let variables: Record<string, string> = {}
	let resources: Record<string, any> = {}
	let displayVariable = false
	let displayResources = false
	let allResultsCollapsed = true
	let flowInputsFiltered: Record<string, any> = {}
	let resultByIdFiltered: Record<string, any> = {}
	let collapsableInitialState:
		| {
				allResultsCollapsed: boolean
				displayVariable: boolean
				displayResources: boolean
		  }
		| undefined

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig, filteredPickableProperties, inputMatches } =
		getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$filteredPickableProperties = { ...pickableProperties }

	$: filterPickableProperties(), updateCollapsable(), search, $inputMatches

	$: suggestedPropsFiltered = $propPickerConfig
		? keepByKey(pickableProperties.priorIds, $propPickerConfig.propName)
		: undefined

	async function loadVariables() {
		variables = Object.fromEntries(
			(
				await VariableService.listVariable({
					workspace: $workspaceStore ?? ''
				})
			).map((variable) => [variable.path, variable.is_secret ? '***' : variable.value ?? ''])
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

	function filterPickableProperties() {
		flowInputsFiltered = pickableProperties.flow_input
		resultByIdFiltered = pickableProperties.priorIds

		if ($inputMatches) {
			if (!$inputMatches.some((match) => match.word === 'flow_input')) {
				flowInputsFiltered = []
			}
			if (!$inputMatches.some((match) => match.word === 'results')) {
				resultByIdFiltered = []
			}
			if ($inputMatches.length == 1) {
				if ($inputMatches[0].word === 'flow_input') {
					let [, ...nestedKeys] = $inputMatches[0].value.split('.')
					flowInputsFiltered = filterNestedObject(flowInputsFiltered, nestedKeys)
				} else if ($inputMatches[0].word === 'results') {
					let [, ...nestedKeys] = $inputMatches[0].value.split('.')
					resultByIdFiltered = filterNestedObject(resultByIdFiltered, nestedKeys)
				}
			}
		}

		if (flowInputsFiltered && search !== EMPTY_STRING) {
			flowInputsFiltered = keepByKey(flowInputsFiltered, search)
		}
		if (resultByIdFiltered && search !== EMPTY_STRING) {
			resultByIdFiltered = keepByKey(resultByIdFiltered, search)
		}

		if ($filteredPickableProperties) {
			resultByIdFiltered && ($filteredPickableProperties.priorIds = resultByIdFiltered)
			flowInputsFiltered && ($filteredPickableProperties.flow_input = flowInputsFiltered)
		}
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
</script>

<div class="flex flex-col h-full !bg-surface rounded overflow-hidden">
	<div class="px-2">
		{#if !notSelectable}
			{#if $propPickerConfig}
				<!-- <Badge large color="blue">
					{`Selected: ${$propPickerConfig?.propName}`}
				</Badge> -->
				<Badge large color="blue">
					{`Mode: ${$propPickerConfig?.insertionMode}`}
				</Badge>
			{:else}
				<Badge large color="blue">&leftarrow; Edit or connect an input</Badge>
			{/if}
		{/if}
		<ClearableInput bind:value={search} placeholder="Search prop..." wrapperClass="py-2" />
	</div>
	<div
		class="overflow-y-auto px-2 pt-2 grow"
		class:bg-surface-secondary={!$propPickerConfig && !notSelectable}
	>
		{#if flowInputsFiltered && Object.keys(flowInputsFiltered).length > 0}
			<div class="flex justify-between items-center space-x-1">
				<span class="font-normal text-sm text-secondary">Flow Input</span>
				<div class="flex space-x-2 items-center" />
			</div>
			<div class="overflow-y-auto mb-2">
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
			<div class="overflow-y-auto mb-2">
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
					<div class="overflow-y-auto mb-2">
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
				<span class="font-normal text-sm text-secondary">All Results</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						{allowCopy}
						pureViewer={!$propPickerConfig}
						collapsed={true}
						json={resultByIdFiltered}
						prefix="results"
						on:select
					/>
				</div>
			{/if}
		{:else}
			{@const json = Object.fromEntries(
				Object.entries(resultByIdFiltered).filter(([k, v]) => k == previousId)
			)}
			{#if previousId && Object.keys(json).length > 0}
				<span class="font-normal text-sm text-secondary">Previous Result</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						{allowCopy}
						pureViewer={!$propPickerConfig}
						{json}
						prefix="results"
						on:select
					/>
				</div>
			{/if}
			{#if pickableProperties.hasResume}
				<span class="font-normal text-sm text-secondary">Resume payloads</span>
				<div class="overflow-y-auto mb-2">
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
				{#if !$inputMatches && suggestedPropsFiltered && Object.keys(suggestedPropsFiltered).length > 0}
					<span class="font-normal text-sm text-secondary">Suggested Results</span>
					<div class="overflow-y-auto mb-2">
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
				{#if Object.keys(resultByIdFiltered).length > 0}
					<div class="overflow-y-auto mb-2">
						<span class="font-normal text-sm text-tertiary">All Results :</span>
						{#if !allResultsCollapsed}
							<Button
								color="light"
								size="xs2"
								variant="border"
								on:click={() => {
									allResultsCollapsed = true
								}}
								wrapperClasses="inline-flex w-fit h-4"
								btnClasses="font-normal text-primary border-nord-300 rounded-[0.275rem]">-</Button
							>
						{/if}

						<ObjectViewer
							{allowCopy}
							pureViewer={!$propPickerConfig}
							bind:collapsed={allResultsCollapsed}
							json={resultByIdFiltered}
							prefix="results"
							on:select
						/>
					</div>
				{/if}
			{/if}
		{/if}

		{#if displayContext}
			{#if !$inputMatches || $inputMatches.some((match) => match.word === 'variable')}
				<div class="overflow-y-auto mb-2">
					<span class="font-normal text-sm text-secondary">Variables :</span>

					{#if displayVariable}
						<Button
							color="light"
							size="xs2"
							variant="border"
							on:click={() => {
								displayVariable = false
							}}
							wrapperClasses="inline-flex w-fit h-4"
							btnClasses="font-normal text-primary border-nord-300 rounded-[0.275rem]">-</Button
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
							wrapperClasses="inline-flex w-fit h-5"
							btnClasses="font-semibold border-nord-300 rounded-[0.275rem] p-1"
						>
							{'{...}'}
						</Button>
					{/if}
				</div>
			{/if}
			{#if !$inputMatches || $inputMatches.some((match) => match.word === 'resource')}
				<div class="overflow-y-auto mb-2">
					<span class="font-normal text-sm text-secondary">Resources :</span>

					{#if displayResources}
						<Button
							color="light"
							size="xs2"
							variant="border"
							on:click={() => {
								displayResources = false
							}}
							wrapperClasses="inline-flex w-fit h-5"
							btnClasses="font-semibold text-primary border-nord-300 rounded-[0.275rem]">-</Button
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
							wrapperClasses="inline-flex w-fit h-5"
							btnClasses="font-semibold border-nord-300 rounded-[0.275rem] p-1"
						>
							{'{...}'}
						</Button>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
</div>
