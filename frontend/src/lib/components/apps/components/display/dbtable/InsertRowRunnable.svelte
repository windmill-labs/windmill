<script lang="ts">
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { AppViewerContext } from '../../../types'
	import type RunnableComponent from '../../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { initOutput } from '../../../editor/appUtils'
	import { type ColumnDef } from './utils'
	import { sendUserToast } from '$lib/toast'
	import { getInsertInput } from './queries/insert'
	import type { DbInput } from '$lib/components/dbTypes'

	interface Props {
		id: string
	}

	let { id }: Props = $props()

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, `${id}_insert`, {
		result: undefined,
		loading: false,
		jobId: undefined
	})

	let runnableComponent: RunnableComponent | undefined = $state()
	let loading = $state(false)
	let input: AppInput | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	export async function insertRow(
		dbInput: DbInput,
		workspace: string | undefined,
		table: string | undefined,
		columns: ColumnDef[],
		values: Record<string, any>
	): Promise<boolean> {
		if (
			(dbInput.type == 'ducklake' && !dbInput.ducklake) ||
			(dbInput.type == 'database' && !dbInput.resourcePath) ||
			!table ||
			!workspace
		) {
			return false
		}

		input = getInsertInput(dbInput, table, columns)

		await tick()

		if (runnableComponent) {
			await runnableComponent?.runComponent(undefined, undefined, undefined, values, {
				onDone: (_x) => {
					dispatch('insert')
					sendUserToast('Row inserted', false)
				},
				onCancel: () => {
					sendUserToast('Error inserting row', true)
				},
				onError: () => {
					sendUserToast('Error inserting row', true)
				}
			})
		}

		return false
	}
</script>

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}_insert`}
	{outputs}
/>
