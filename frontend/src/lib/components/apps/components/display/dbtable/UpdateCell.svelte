<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { createUpdatePostgresInput } from './utils'

	export let id: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let runnableWrapper: RunnableWrapper
	let loading = false

	let input: AppInput | undefined = undefined

	export async function triggerUpdate(
		resource: string,
		table: string | undefined,
		rowIndex: number,
		column: string,
		value: string
	) {
		input = createUpdatePostgresInput(resource, table, rowIndex, column, value)

		await tick()

		if (!runnableComponent) {
			runnableWrapper?.handleSideEffect(true)
		} else {
			await runnableComponent?.runComponent()
		}
	}
</script>

<RunnableWrapper
	bind:this={runnableWrapper}
	recomputeIds={[id]}
	bind:runnableComponent
	bind:loading
	componentInput={input}
	{id}
	autoRefresh={false}
	render={true}
	{outputs}
/>
