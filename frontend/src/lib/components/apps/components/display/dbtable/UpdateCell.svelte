<script lang="ts">
	import { getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { createUpdatePostgresInput, type TableMetadata } from './utils'

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
		table: string | undefined,
		rowIndex: number,
		column: string,
		value: string,
		data: Record<string, any>,
		tableMetaData: TableMetadata | undefined = undefined
	) {
		const primaryKey = tableMetaData?.find((column) => column.isprimarykey)?.columnname
		const primaryValue = primaryKey ? data[primaryKey] : undefined

		input = createUpdatePostgresInput(resource, table, column, value, primaryKey, primaryValue)

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
