<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef, type DbType } from './utils'
	import { getCountInput } from './queries/count'

	export let id: string
	export let table: string | undefined
	export let resource: string | undefined
	export let renderCount: number
	export let quicksearch: string
	export let resourceType: string
	export let columnDefs: ColumnDef[]
	export let whereClause: string | undefined

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_count`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let loading = false
	let input: AppInput | undefined = undefined
	let lastTableCount: string | undefined = undefined
	let renderCountLast = -1
	let quicksearchLast: string | undefined = undefined

	let localColumnDefs = columnDefs
	let lastTable = table

	$: {
		if (table !== lastTable) {
			localColumnDefs = []
			lastTable = table
		}
	}

	$: table && renderCount != undefined && quicksearch != undefined && computeCount()

	async function computeCount() {
		if (
			lastTableCount === table &&
			renderCount == renderCountLast &&
			quicksearch == quicksearchLast
		)
			return
		if (table != undefined && resource !== undefined) {
			renderCountLast = renderCount
			lastTableCount = table
			quicksearchLast = quicksearch
			await getCount(resource, table, quicksearch)
		}
	}

	async function getCount(resource: string, table: string, quicksearch: string) {
		input = getCountInput(resource, table, resourceType as DbType, localColumnDefs, whereClause)

		await tick()

		if (runnableComponent) {
			await runnableComponent?.runComponent(undefined, undefined, undefined, {
				quicksearch
			})
		}
	}
</script>

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_count`}
	{outputs}
/>
