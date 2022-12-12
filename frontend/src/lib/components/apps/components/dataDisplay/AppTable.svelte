<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext, BaseAppComponent, ButtonComponent } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'
	import DebouncedInput from '../helpers/DebouncedInput.svelte'
	import AppButton from '../buttons/AppButton.svelte'
	import type { AppInput } from '../../inputType'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]

	const { worldStore, staticOutputs: staticOutputsStore } =
		getContext<AppEditorContext>('AppEditorContext')

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

	let search: 'Frontend' | 'Backend' | 'Disabled' = 'Disabled'
	let pagination: boolean | undefined = undefined

	let page = 1
	let searchValue = ''
	$: result = [] as Array<Record<string, any>>
	$: headers = Array.from(new Set(result.flatMap((row) => Object.keys(row))))
	$: extraQueryParams = search === 'Backend' ? { search: searchValue, page } : { page }

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value.toString().includes(searchValue))
		)
	}

	let filteredResult: Array<Record<string, any>> = []

	$: search === 'Frontend' && (filteredResult = searchInResult(result, searchValue))
	$: (search === 'Backend' || search === 'Disabled') && (filteredResult = result)
</script>

<InputValue input={configuration.search} bind:value={search} />
<InputValue input={configuration.pagination} bind:value={pagination} />

<RunnableWrapper bind:componentInput {id} bind:result>
	<div class="gap-2 flex flex-col mt-2">
		{#if search !== 'Disabled'}
			<div>
				<div>
					<DebouncedInput placeholder="Search..." bind:value={searchValue} />
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
					{#each filteredResult as row, rowIndex (rowIndex)}
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
								{#if actionButtons?.length > 0}
									<div class="center-center gap-2">
										{#each actionButtons as props}
											<AppButton
												{...props}
												extraQueryParams={{ row }}
												bind:componentInput={props.componentInput}
												bind:staticOutputs={$staticOutputsStore[props.id]}
											/>
										{/each}
									</div>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if pagination}
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
</RunnableWrapper>
