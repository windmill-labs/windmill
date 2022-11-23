<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { Output } from '../rx'
	import type { AppEditorContext, ComponentInputsSpec, InputsSpec } from '../types'
	import InputValue from './helpers/InputValue.svelte'
	import DebouncedInput from './helpers/DebouncedInput.svelte'
	import RunnableComponent from './helpers/RunnableComponent.svelte'

	export let id: string
	export let inputs: InputsSpec
	export let path: string | undefined = undefined
	export let runType: 'script' | 'flow' | undefined = undefined
	export let inlineScriptName: string | undefined = undefined
	export let componentInputs: ComponentInputsSpec

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	export const staticOutputs: string[] = ['selectedRow', 'loading', 'result']

	$: outputs = $worldStore?.outputsById[id] as {
		selectedRow: Output<any>
	}

	let selectedRowIndex = -1

	function toggleRow(row: Record<string, any>, rowIndex: number) {
		if (selectedRowIndex === rowIndex) {
			selectedRowIndex = -1
			outputs.selectedRow.set(null)
		} else {
			selectedRowIndex = rowIndex
			outputs?.selectedRow.set(row)
		}
	}

	let searchEnabledValue: boolean | undefined = undefined
	let paginationEnabled: boolean | undefined = undefined

	let page = 1
	let search = ''

	let result: Array<Record<string, any>> = []
	$: headers = Object.keys(result[0] || {}) || []

	const extraQueryParams = { search, page }

	export const reservedKeys: string[] = Object.keys(extraQueryParams)
</script>

<InputValue input={componentInputs.searchEnabled} bind:value={searchEnabledValue} />
<InputValue input={componentInputs.paginationEnabled} bind:value={paginationEnabled} />

<RunnableComponent
	{id}
	{path}
	{runType}
	{inlineScriptName}
	bind:inputs
	bind:result
	extraQueryParams={{ search, page }}
>
	<div class="gap-2 flex flex-col mt-2">
		{#if searchEnabledValue}
			<div>
				<div>
					<DebouncedInput placeholder="Search..." bind:value={search} />
				</div>
			</div>
		{/if}
		<div class="flex flex-col">
			<table class="divide-y divide-gray-300 border">
				{#if headers}
					<thead class="bg-gray-50">
						<tr>
							{#each headers as header}
								<th
									scope="col"
									class="px-4 py-2 text-left text-xs font-medium text-gray-500 tracking-wider"
								>
									{header.replace(/([A-Z]+)*([A-Z][a-z])/g, '$1 $2')}
								</th>
							{/each}
							<th scope="col" class="relative py-2 px-4">
								<span class="sr-only">Edit</span>
							</th>
						</tr>
					</thead>
				{/if}
				<tbody class="divide-y divide-gray-200 bg-white">
					{#each result as row, rowIndex (rowIndex)}
						<tr
							class={classNames(
								selectedRowIndex === rowIndex ? 'bg-blue-100 hover:bg-blue-200' : 'hover:bg-blue-50'
							)}
							on:click={() => toggleRow(row, rowIndex)}
						>
							{#each headers as header}
								<td class="px-4 py-2 whitespace-nowrap text-sm text-gray-900">
									{row[header]}
								</td>
							{/each}
							<td class="relative whitespace-nowrap px-4 py-2 text-right ">
								{#if false}
									<Button color="blue" size="xs" variant="contained">Edit</Button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if paginationEnabled}
			<div class="flex flex-row gap-2">
				<Button
					on:click={() => {
						page = page - 1
					}}
					color="light"
					size="xs"
					variant="border"
					disabled={page === 1}
				>
					Previous
				</Button>
				<Button
					on:click={() => {
						page = page + 1
					}}
					color="light"
					size="xs"
					variant="border">Next</Button
				>
			</div>
		{/if}
	</div>
</RunnableComponent>
