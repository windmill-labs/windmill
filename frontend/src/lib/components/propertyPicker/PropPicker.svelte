<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getContext } from 'svelte'
	import { Button } from '../common'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import type { PickableProperties } from '../flows/previousResults'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'

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

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	let flowInputsFiltered = pickableProperties.flow_input
	let resultByIdFiltered = pickableProperties.priorIds

	let timeout: NodeJS.Timeout
	function onSearch(search: string) {
		clearTimeout(timeout)
		setTimeout(() => {
			flowInputsFiltered =
				search === EMPTY_STRING
					? pickableProperties.flow_input
					: keepByKey(pickableProperties.flow_input, search)

			resultByIdFiltered =
				search === EMPTY_STRING
					? pickableProperties.priorIds
					: keepByKey(pickableProperties.priorIds, search)

			console.log(resultByIdFiltered, search)
		}, 50)
	}

	$: suggestedPropsFiltered = $propPickerConfig
		? keepByKey(pickableProperties.priorIds, $propPickerConfig.propName)
		: undefined
	$: search != undefined && onSearch(search)

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

<div class="flex flex-col h-full rounded overflow-hidden">
	<div class="px-2 py-2">
		<ClearableInput bind:value={search} placeholder="Search prop..." />
	</div>
	<div class="overflow-y-auto px-2 pt-2 grow">
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
			{#if previousId}
				<span class="font-normal text-sm text-secondary">Previous Result</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						{allowCopy}
						pureViewer={!$propPickerConfig}
						json={Object.fromEntries(
							Object.entries(resultByIdFiltered).filter(([k, v]) => k == previousId)
						)}
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
				<div class="overflow-y-auto mb-2">
					<span class="font-normal text-sm text-secondary">All Results</span>
					{#if !allResultsCollapsed}
						<Button
							color="light"
							size="xs2"
							variant="contained"
							on:click={() => {
								allResultsCollapsed = true
							}}
							wrapperClasses="inline-flex w-fit h-4"
							btnClasses="font-normal text-primary rounded-[0.275rem]">-</Button
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

		{#if displayContext}
			<div class="overflow-y-auto mb-2">
				<span class="font-normal text-sm text-secondary">Variables:</span>

				{#if displayVariable}
					<Button
						color="light"
						size="xs2"
						variant="border"
						on:click={() => {
							displayVariable = false
						}}
						wrapperClasses="inline-flex w-fit h-4"
						btnClasses="font-normal text-primary rounded-[0.275rem]">-</Button
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
						wrapperClasses="inline-flex w-fit h-4"
						btnClasses="font-normal rounded-[0.275rem] p-1"
					>
						{'{...}'}
					</Button>
				{/if}
			</div>

			<div class="overflow-y-auto mb-2">
				<span class="font-normal text-sm text-secondary">Resources:</span>

				{#if displayResources}
					<Button
						color="light"
						size="xs2"
						variant="border"
						on:click={() => {
							displayResources = false
						}}
						wrapperClasses="inline-flex w-fit h-4"
						btnClasses="font-normal text-primary rounded-[0.275rem]">-</Button
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
						wrapperClasses="inline-flex w-fit h-4"
						btnClasses="font-normal rounded-[0.275rem] p-1"
					>
						{'{...}'}
					</Button>
				{/if}
			</div>
		{/if}
	</div>
</div>
