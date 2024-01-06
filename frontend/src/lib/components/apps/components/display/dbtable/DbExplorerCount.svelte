<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { getCountPostgresql } from './utils'

	export let id: string
	export let table: string
	export let resource: string
	export let renderCount: number
	export let quicksearch: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_count`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let loading = false

	let input: AppInput | undefined = undefined

	$: table && renderCount != undefined && quicksearch != undefined && computeCount()

	let lastTableCount = ''
	let renderCountLast = -1
	let quicksearchLast: string | undefined = undefined
	async function computeCount() {
		if (
			lastTableCount === table &&
			renderCount == renderCountLast &&
			quicksearch == quicksearchLast
		)
			return
		if (table != '' && resource != '') {
			renderCountLast = renderCount
			lastTableCount = table
			quicksearchLast = quicksearch
			await getCount(resource, table, quicksearch)
		}
	}

	async function getCount(resource: string, table: string, quicksearch: string) {
		input = getCountPostgresql(resource, table)

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
