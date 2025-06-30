<script lang="ts">
	import { getContext, tick, untrack } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef, type DbType } from './utils'
	import { getCountInput } from './queries/count'

	interface Props {
		id: string
		table: string | undefined
		resource: string | undefined
		renderCount: number
		quicksearch: string
		resourceType: string
		columnDefs: ColumnDef[]
		whereClause: string | undefined
	}

	let {
		id,
		table,
		resource,
		renderCount,
		quicksearch,
		resourceType,
		columnDefs,
		whereClause
	}: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_count`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state()
	let loading = $state(false)
	let input: AppInput | undefined = $state(undefined)
	let lastTableCount: string | undefined = undefined
	let renderCountLast = -1
	let quicksearchLast: string | undefined = undefined

	let localColumnDefs = columnDefs
	let lastTable = $state(table)

	function onTableChange() {
		if (table !== lastTable) {
			lastTable = table
			localColumnDefs = []
		}
	}

	export async function computeCount(forceCompute?: boolean | undefined) {
		if (!forceCompute) {
			if (
				lastTableCount === table &&
				renderCount == renderCountLast &&
				quicksearch == quicksearchLast
			) {
				return
			}
		}

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
	$effect(() => {
		lastTable != undefined && table && untrack(() => onTableChange())
	})
	$effect(() => {
		table && renderCount != undefined && quicksearch != undefined && untrack(() => computeCount())
	})
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
