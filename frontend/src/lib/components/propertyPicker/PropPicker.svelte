<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import { Badge, Button } from '../common'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'
	import { createEventDispatcher } from 'svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import type { PickableProperties } from '../flows/previousResults'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'

	export let pickableProperties: PickableProperties
	export let displayContext = true
	export let notSelectable: boolean
	export let error: boolean = false

	$: previousId = pickableProperties?.previousId
	let variables: Record<string, string> = {}
	let resources: Record<string, any> = {}
	let displayVariable = false
	let displayResources = false
	let allResultsCollapsed = true
	const dispatch = createEventDispatcher()

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: flowInputsFiltered =
		search === EMPTY_STRING
			? pickableProperties.flow_input
			: keepByKey(pickableProperties.flow_input, search)

	$: resultByIdFiltered =
		search === EMPTY_STRING
			? pickableProperties.priorIds
			: keepByKey(pickableProperties.priorIds, search)

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
</script>

<div class="flex flex-col h-full">
	<div class="px-2">
		{#if !notSelectable}
			<div class="flex flex-row space-x-1">
				<Badge large color="blue">&leftarrow; Edit or connect an input</Badge>
			</div>
		{/if}
		<ClearableInput bind:value={search} placeholder="Search prop..." wrapperClass="py-2" />
	</div>
	<div
		class="overflow-y-auto px-2 pt-2 grow"
		class:bg-surface-secondary={!$propPickerConfig && !notSelectable}
	>
		<div class="flex justify-between items-center space-x-1">
			<span class="font-normal text-sm text-secondary">Flow Input</span>
			<div class="flex space-x-2 items-center" />
		</div>
		<div class="overflow-y-auto mb-2">
			<ObjectViewer
				allowCopy={false}
				pureViewer={!$propPickerConfig}
				json={flowInputsFiltered}
				on:select={(e) => {
					dispatch(
						'select',
						e.detail?.startsWith('[') ? `flow_input${e.detail}` : `flow_input.${e.detail}`
					)
				}}
			/>
		</div>
		{#if error}
			<span class="font-normal text-sm text-secondary">Error</span>
			<div class="overflow-y-auto mb-2">
				<ObjectViewer
					allowCopy={false}
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
							allowCopy={false}
							pureViewer={!$propPickerConfig}
							collapsed={false}
							json={suggestedPropsFiltered}
							on:select={(e) => {
								dispatch('select', `results.${e.detail}`)
							}}
						/>
					</div>
				{/if}
				<span class="font-normal text-sm text-secondary">All Results</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						collapsed={true}
						json={resultByIdFiltered}
						on:select={(e) => {
							dispatch('select', `results.${e.detail}`)
						}}
					/>
				</div>
			{/if}
		{:else}
			{#if previousId}
				<span class="font-normal text-sm text-secondary">Previous Result</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						json={Object.fromEntries(
							Object.entries(resultByIdFiltered).filter(([k, v]) => k == previousId)
						)}
						on:select={(e) => {
							dispatch('select', `results.${e.detail}`)
						}}
					/>
				</div>
			{/if}
			{#if pickableProperties.hasResume}
				<span class="font-normal text-sm text-secondary">Resume payloads</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						json={{
							resume: 'The resume payload',
							resumes: 'All resume payloads from all approvers',
							approvers: 'The list of approvers'
						}}
						on:select={(e) => {
							dispatch('select', `${e.detail}`)
						}}
					/>
				</div>
			{/if}
			{#if Object.keys(pickableProperties.priorIds).length > 0}
				{#if suggestedPropsFiltered && Object.keys(suggestedPropsFiltered).length > 0}
					<span class="font-normal text-sm text-secondary">Suggested Results</span>
					<div class="overflow-y-auto mb-2">
						<ObjectViewer
							allowCopy={false}
							pureViewer={!$propPickerConfig}
							collapsed={false}
							json={suggestedPropsFiltered}
							on:select={(e) => {
								dispatch('select', `results.${e.detail}`)
							}}
						/>
					</div>
				{/if}
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
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						bind:collapsed={allResultsCollapsed}
						json={resultByIdFiltered}
						on:select={(e) => {
							dispatch('select', `results.${e.detail}`)
						}}
					/>
				</div>
			{/if}
		{/if}

		{#if displayContext}
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
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						rawKey={true}
						json={variables}
						on:select={(e) => dispatch('select', `variable('${e.detail}')`)}
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
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						rawKey={true}
						json={resources}
						on:select={(e) => dispatch('select', `resource('${e.detail}')`)}
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
	</div>
</div>
