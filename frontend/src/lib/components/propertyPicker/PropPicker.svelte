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
				{#if $propPickerConfig}
					<Badge large color="blue">
						{`Selected: ${$propPickerConfig?.propName}`}
					</Badge>
					<Badge large color="blue">
						{`Mode: ${$propPickerConfig?.insertionMode}`}
					</Badge>
				{:else}
					<Badge large color="blue">&leftarrow; Edit or connect an input</Badge>
				{/if}
			</div>
		{/if}
		<ClearableInput bind:value={search} placeholder="Search prop..." wrapperClass="py-2" />
	</div>
	<div
		class="overflow-y-auto px-2 pt-2 grow"
		class:bg-surface-secondary={!$propPickerConfig && !notSelectable}
	>
		<div class="flex justify-between items-center space-x-1">
			<span class="font-bold text-sm">Flow Input</span>
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
			<span class="font-bold text-sm">Error</span>
			<div class="overflow-y-auto mb-2">
				<ObjectViewer
					allowCopy={false}
					pureViewer={!$propPickerConfig}
					json={{
						error: {
							message: 'The error message',
							name: 'The error name',
							stack: 'The error stack'
						}
					}}
					on:select
				/>
			</div>
		{:else}
			{#if previousId}
				<span class="font-bold text-sm">Previous Result</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						topLevelNode
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
				<span class="font-bold text-sm">Resume payloads</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						topLevelNode
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
					<span class="font-bold text-sm">Suggested Results</span>
					<div class="overflow-y-auto mb-2">
						<ObjectViewer
							allowCopy={false}
							topLevelNode
							pureViewer={!$propPickerConfig}
							collapsed={false}
							json={suggestedPropsFiltered}
							on:select={(e) => {
								dispatch('select', `results.${e.detail}`)
							}}
						/>
					</div>
				{/if}
				<span class="font-bold text-sm">All Results</span>
				<div class="overflow-y-auto mb-2">
					<ObjectViewer
						allowCopy={false}
						topLevelNode
						pureViewer={!$propPickerConfig}
						collapsed={true}
						json={resultByIdFiltered}
						on:select={(e) => {
							dispatch('select', `results.${e.detail}`)
						}}
					/>
				</div>
			{/if}
		{/if}

		{#if displayContext}
			<span class="font-bold text-sm">Variables </span>
			<div class="overflow-y-auto mb-2">
				{#if displayVariable}
					<div class="flex">
						<Button
							color="light"
							size="xs"
							variant="border"
							on:click={() => {
								displayVariable = false
							}}>-</Button
						>
					</div>
					<ObjectViewer
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						rawKey={true}
						json={variables}
						on:select={(e) => dispatch('select', `variable('${e.detail}')`)}
					/>
				{:else}
					<button
						class="border border-blue-600 key font-normal rounded hover:bg-blue-100 px-1"
						on:click={async () => {
							await loadVariables()
							displayVariable = true
						}}>{'{...}'}</button
					>
				{/if}
			</div>
			<span class="font-bold text-sm">Resources</span>
			<div class="overflow-y-auto mb-2">
				{#if displayResources}
					<Button
						color="light"
						variant="border"
						size="xs"
						on:click={() => {
							displayResources = false
						}}>-</Button
					>
					<ObjectViewer
						allowCopy={false}
						pureViewer={!$propPickerConfig}
						rawKey={true}
						json={resources}
						on:select={(e) => dispatch('select', `resource('${e.detail}')`)}
					/>
				{:else}
					<button
						class="border border-blue-600 px-1 key font-normal rounded hover:bg-blue-100"
						on:click={async () => {
							await loadResources()
							displayResources = true
						}}>{'{...}'}</button
					>
				{/if}
			</div>
		{/if}
	</div>
</div>
