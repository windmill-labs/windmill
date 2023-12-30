<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { createUpdatePostgresInput, type ColumnMetadata } from './utils'

	export let id: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `update-${id}`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent
	let loading = false

	let input: AppInput | undefined = undefined

	export async function triggerUpdate(
		resource: string,
		table: string,
		rowIndex: number,
		column: ColumnMetadata,
		columns: ColumnMetadata[],
		value: string,
		data: Record<string, any>,
		oldValue: string | undefined = undefined
	) {
		// const datatype = tableMetaData?.find((column) => column.isprimarykey)?.datatype
		const clonnedData = { ...data }
		clonnedData[column.field] = oldValue

		input = createUpdatePostgresInput(resource, table, column, columns, value, clonnedData)

		await tick()

		if (runnableComponent) {
			await runnableComponent?.runComponent()
		}
	}
</script>

<RunnableWrapper
	bind:runnableComponent
	bind:loading
	recomputeIds={[id]}
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`update-${id}`}
	{outputs}
/>
