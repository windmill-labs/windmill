<script lang="ts">
	import { ResourceService, VariableService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { Button } from '../common'
	import type { PropPickerWrapperContext } from '../flows/propPicker/PropPickerWrapper.svelte'
	import { createEventDispatcher } from 'svelte'

	import ObjectViewer from './ObjectViewer.svelte'
	import { keepByKey } from './utils'
	import { flowStateStore } from '../flows/flowState'
	import { flowIds } from '../flows/flowStore'

	export let pickableProperties: Object = {}
	export let displayContext = true
	export let priorId: string | undefined
	let variables: Record<string, string> = {}
	let resources: Record<string, any> = {}
	let displayVariable = false
	let displayResources = false

	const dispatch = createEventDispatcher()

	const EMPTY_STRING = ''
	let search = ''

	const { propPickerConfig, clearFocus } = getContext<PropPickerWrapperContext>('PropPickerWrapper')

	$: propsFiltered =
		search === EMPTY_STRING ? pickableProperties : keepByKey(pickableProperties, search)

	let priorIds = {}
	$: {
		if (priorId) {
			const allState = $flowStateStore
			priorIds = Object.fromEntries(
				Object.entries(allState)
					.filter(
						(o) =>
							$flowIds.includes(o[0]) && $flowIds.indexOf(o[0]) <= $flowIds.indexOf(priorId ?? '')
					)
					.map((o) => [o[0], o[1].previewResult])
			)
		}
	}
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

<input
	type="text"
	bind:value={search}
	class="bg-gray-50 mt-1 border border-gray-300 text-gray-900 text-sm rounded-lg block px-2 mb-2 w-full"
	placeholder="Search prop..."
/>
<div class="flex justify-between items-center space-x-1">
	<span class="font-bold text-sm">Step Context</span>
	<div class="flex space-x-2 items-center">
		{#if $propPickerConfig}
			<span
				class="flex items-center bg-blue-100 text-blue-800 text-xs font-semibold px-2 py-1 rounded dark:bg-green-200 dark:text-green-900"
			>
				{`Selected: ${$propPickerConfig?.propName}`}
			</span>
			<span
				class="flex items-center bg-blue-100 text-blue-800 text-xs font-semibold px-2 py-1 rounded dark:bg-green-200 dark:text-green-900"
			>
				{`Mode: ${$propPickerConfig?.insertionMode}`}
			</span>
			<button
				class="border px-2 py-1 text-xs rounded-md flex items-center hover:bg-gray-50 hover:text-gray-900"
				on:click={() => clearFocus()}
			>
				<Icon data={faClose} class="mr-2" scale={0.8} />
				Deselect
			</button>
		{/if}
	</div>
</div>
<div class="overflow-y-auto mb-2">
	<ObjectViewer json={propsFiltered} on:select />
</div>
{#if priorId}
	<span class="font-bold text-sm">Result by id</span>
	<div class="overflow-y-auto mb-2">
		<ObjectViewer
			collapsed={true}
			json={priorIds}
			on:select={(e) => dispatch('select', `result_by_id('${e.detail}')`)}
		/>
	</div>
{/if}
{#if displayContext}
	<span class="font-bold text-sm">Variables </span>
	<div class="overflow-y-auto mb-2">
		{#if displayVariable}
			<Button
				color="light"
				size="xs"
				on:click={() => {
					displayVariable = false
				}}>(-)</Button
			>
			<ObjectViewer
				rawKey={true}
				json={variables}
				on:select={(e) => dispatch('select', `variable('${e.detail}')`)}
			/>
		{:else}
			<Button
				color="light"
				size="xs"
				on:click={async () => {
					await loadVariables()
					displayVariable = true
				}}>{'{...}'}</Button
			>
		{/if}
	</div>
	<span class="font-bold text-sm">Resources</span>
	<div class="overflow-y-auto mb-2">
		{#if displayResources}
			<Button
				color="light"
				size="xs"
				on:click={() => {
					displayResources = false
				}}>(-)</Button
			>
			<ObjectViewer
				rawKey={true}
				json={resources}
				on:select={(e) => dispatch('select', `resource('${e.detail}')`)}
			/>
		{:else}
			<Button
				color="light"
				size="xs"
				on:click={async () => {
					await loadResources()
					displayResources = true
				}}>{'{...}'}</Button
			>
		{/if}
	</div>
{/if}
