<script lang="ts">
	import AgGridSvelte from 'ag-grid-svelte/AgGridSvelte.svelte'
	import 'ag-grid-community/styles/ag-grid.css'
	import 'ag-grid-community/styles/ag-theme-alpine.css'

	import { getContext, onMount } from 'svelte'
	import type { Output } from '../../../rx'
	import type { AppEditorContext } from '../../../types'
	import InputValue from '../../helpers/InputValue.svelte'
	import type { AppInput } from '../../../inputType'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { isObject } from '$lib/utils'

	import Alert from '$lib/components/common/alert/Alert.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: Record<string, AppInput>
	export let initializing: boolean | undefined = undefined

	export const staticOutputs: string[] = ['selectedRow', 'loading', 'result', 'selectedRowIndex']

	let result: Record<string, any>[] | undefined = undefined

	const {
		worldStore,
		staticOutputs: staticOutputsStore,
		selectedComponent
	} = getContext<AppEditorContext>('AppEditorContext')

	let selectedRowIndex = -1

	function toggleRow(row: Record<string, any>, rowIndex: number) {
		if (selectedRowIndex !== rowIndex) {
			selectedRowIndex = rowIndex
			outputs?.selectedRow.set(row.original)
			outputs?.selectedRowIndex.set(rowIndex)
		}
	}

	let mounted = false
	onMount(() => {
		mounted = true
	})

	$: selectedRowIndex === -1 &&
		Array.isArray(result) &&
		result.length > 0 &&
		// We need to wait until the component is mounted so the world is created
		mounted &&
		outputs &&
		toggleRow({ original: result[0] }, 0)

	$: outputs = $worldStore?.outputsById[id] as {
		selectedRowIndex: Output<number>
		selectedRow: Output<any>
		result: Output<any>
	}

	$: outputs?.result?.set(result)

	let clientHeight
	let clientWidth

	let columnDefs: any = undefined

	let allEditable: boolean | undefined = undefined

	function onCellValueChanged(event) {
		if (result) {
			let dataCell = event.newValue
			try {
				dataCell = JSON.parse(dataCell)
			} catch (e) {}
			result[event.node.rowIndex][event.column.colId] = dataCell
		}
	}
</script>

<InputValue {id} input={configuration.columnDefs} bind:value={columnDefs} />
<InputValue {id} input={configuration.allEditable} bind:value={allEditable} />

<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
	{#if Array.isArray(result) && result.every(isObject)}
		<div
			class="border border-gray-300 shadow-sm divide-y divide-gray-300  flex flex-col h-full"
			bind:clientHeight
			bind:clientWidth
		>
			<div
				on:pointerdown|stopPropagation={() => {
					$selectedComponent = id
				}}
				style:height="{clientHeight}px"
				style:width="{clientWidth}px"
				class="ag-theme-alpine"
			>
				<AgGridSvelte
					bind:rowData={result}
					{columnDefs}
					defaultColDef={{ flex: 1, editable: allEditable, onCellValueChanged }}
				/>
			</div>
		</div>
	{:else if result != undefined}
		<Alert title="Parsing issues" type="error" size="xs">
			The result should be an array of objects
		</Alert>
	{/if}
</RunnableWrapper>
